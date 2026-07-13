# mneme MCP tools advertise an empty `inputSchema` (`{}`) — breaks Anthropic tool-calling for consumers

**Filed:** 2026-07-13 (during thoth 0.34.2)
**Severity:** high (breaks the whole agentic tool-calling turn for any Anthropic-backed consumer)
**Status:** ✅ **RESOLVED in mneme 1.1.1** — registration now serializes and sends each tool's `inputSchema` (new
`_mcp_schema_json` in `src/mcp_server.cyr`), verified live: daimon advertises typed `mneme_*` schemas and a thoth
full-registry agentic turn completes. thoth 0.34.2 also hardened its request builder to tolerate any empty/typeless
schema (defense-in-depth). Two follow-ups noted below.

## Follow-ups (not blocking; noted during the fix)

1. **daimon's registration parser is order-sensitive.** `POST /v1/mcp/tools` returns `400 {"error":"missing
   callback_url"}` when `callback_url` appears *after* a nested object (e.g. `inputSchema`) in the body — it scans
   flatly rather than parsing JSON. mneme 1.1.1 works around it by emitting `callback_url` before `inputSchema`
   (matching `stack.sh`'s registration order), but daimon should parse the body as JSON so field order doesn't
   matter. (daimon-side.)
2. **mneme's `ToolSchema` only carries `type` + required NAMES**, not per-arg types/descriptions or optional args
   (`mcp_protocol.cyr`). 1.1.1 defaults every required arg to `{"type":"string"}` (correct for all current
   note-tool args) and cannot advertise optional args (e.g. `create_note`'s `tags`). A richer `ToolSchema` (named
   properties with types + an optional set) would let the model see full typed signatures. (mneme-side, future.)

## Symptom

Queried through daimon's registry (`GET /v1/mcp/tools`), every `mneme_*` tool is advertised with an **empty**
`inputSchema`:

```json
{ "name": "mneme_create_note",
  "description": "Create a new note in the Mneme knowledge base with title, content, and optional tags",
  "inputSchema": {} }
```

This is despite `src/mcp_protocol.cyr` **defining** a real schema for the tool, e.g. around line 88:

```
mneme_mcp_tool_def_new(str_from("mneme_create_note"), str_from("Create a new note …"),
                       _mcp_obj(_mcp_req2("title", "content")))
```

So the typed schema (`title`/`content`/`tags`/`query`/…) that mneme constructs is **lost** somewhere between the
`ToolDef.input_schema` field and what a consumer sees over MCP — either mneme's `tools/list` serialization drops it,
or mneme's self-registration to daimon sends `{}`. (mneme's own `:8100/mcp` `tools/list` did not answer in the local
stack, so I could not isolate mneme-serialization vs the daimon-registration hop from the outside — please check both,
starting with how `input_schema` is serialized into the `tools/list` result / the registration payload.)

## Impact

**Anthropic requires** each tool's `input_schema` to be `{"type":"object", …}`. A bare `{}` is invalid. An
OpenAI-compatible gateway (hoosh) forwards `parameters: {}` → `input_schema: {}` verbatim, and Anthropic then rejects
the **entire** request — not just the one tool. Downstream this surfaces as:

- **streaming** (`stream=true`): an empty completion — *"response had neither tool calls nor content."*
- **block** (`stream=false`): **HTTP 502**.

So a single mneme tool with `{}` poisons every agentic turn whenever mneme is in the registry (thoth saw all 22
registry tools fail together). It reads like a "large tools payload" problem but is not — it is this one invalid
schema.

## Downstream workaround (already shipped)

thoth 0.34.2 made its OpenAI request builder tolerant: when a tool's `inputSchema` is absent, empty, or lacks a
top-level `"type"`, thoth emits the permissive `{"type":"object"}` instead of the bare `{}`. That unblocks the loop,
but the model then gets a **freeform-object** schema for the `mneme_*` tools (no typed `title`/`content`/… args), so
it can't be guided to the right arguments. The proper fix — real schemas advertised from here — restores typed args.

## Suggested fix

Ensure the `input_schema` mneme builds in `mcp_protocol.cyr` is actually serialized into the MCP `tools/list`
result (and the daimon-registration payload) as a real `{"type":"object","properties":{…},"required":[…]}` object,
not flattened to `{}`. Verify with `tools/list` over mneme's own MCP endpoint and via daimon's `/v1/mcp/tools`.
