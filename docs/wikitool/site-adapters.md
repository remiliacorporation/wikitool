# Site adapters

A site adapter is an explicit project-relative TOML file. Release archives include generic and
Remilia Wiki templates under `site_adapters/`, but catalog presence does not activate one. Copy a
template into the project, then validate and select it during initialization:

```bash
cp -R /path/to/unpacked-wikitool/site_adapters/remilia-wiki site-adapter
wikitool init --adapter-path site-adapter/site-adapter.toml
```

This records the following in `.wikitool/config.toml`:

```toml
[adapter]
path = "site-adapter/site-adapter.toml"
```

The configured path must be project-relative, and its canonical target must remain under the
project root. Absolute paths, `..` escapes, and symlinks to external policy trees are rejected so
snapshots and isolated evaluations cannot depend on mutable files outside the project.

Without this section Wikitool uses the embedded `mediawiki-generic` adapter. It does not search executable ancestors or silently inherit a branded policy.

## Machine policy

The current strict document identity begins with:

```toml
schema_version = "site_adapter_v2"
adapter_id = "example-wiki"
docs_profile = "mw-1.44-authoring"
guidance_documents = []
```

`adapter_id` identifies the complete site policy. The contract does not imply adapter inheritance;
it is deliberately distinct from documentation-corpus profiles, authoring-surface profiles, and
projection target profiles.

The adapter can declare:

- short-description and article-quality mechanics;
- required appendices and reference template;
- citation template families and named-reference behavior;
- deterministic exact-host or subdomain rules that request source review;
- infobox preferences and category hints;
- mechanically decidable style constraints and placeholder artifacts;
- target extension and module availability contracts;
- relative supplemental guidance documents.

Unknown fields are rejected. Source-review hosts must be normalized lowercase hostnames; URL
substrings, schemes, paths, ports, and wildcards are not accepted as host rules. Guidance paths
must remain within the adapter directory. Wikitool hashes and exposes supplemental Markdown but
never parses it as executable policy.

Source-review rules are routing signals, not universal bans. Their reasons should tell the review skill what to inspect. Semantic exceptions stay in review findings rather than being hidden in substring logic.

Release packaging always places the versioned built-in catalog under `site_adapters/generic/` and
`site_adapters/remilia-wiki/`. An optional host bundle goes under `site_adapters/project/` after
validating the policy and copying only its declared resources. Presence in a release archive does
not activate any adapter: the installed project must place the selected adapter at a
project-relative path and record that path in `.wikitool/config.toml`.

## Supplemental guidance

Keep target names, relationships, local source rules, category governance,
visual-subject conventions and extension semantics in adapter Markdown. Load
only guidance relevant to the current task. A host supplement must not redefine
retrieval observations as evidence or claim that acceptance authenticates an editor.

Put category definitions and the boundary around taxonomy creation here, not
in a portable skill's hard-coded list. `preferred_categories` contains hints;
it does not enforce a live approved vocabulary. Membership counts, existing
category tags and template catalog families do not imply editorial approval.

All declared guidance contributes to the adapter identity even when an agent
does not need to read every document for a particular task. Changing guidance
invalidates acceptance bound to the previous policy; do not copy old decisions
onto the new identity. A copied adapter remains owned by the consuming project
and does not update automatically when the distribution changes.

## Portability test

A standalone Wikitool checkout with no adapter should initialize offline, expose the generic adapter, and contain no target-wiki URL, relationship, template, category, or source verdict. A host project should regain all intended local behavior only after its explicit adapter is configured.
