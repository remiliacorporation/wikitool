# Wikitool test guarantee review, 2026-09-29

This review covers the article-lint test file, the catalog timing exercise, and the new `templates show` CLI contract. It does not assign verdicts to the rest of the workspace suite. The CI gate runs `cargo test --workspace --all-features`; the portable CLI gate runs Wikitest on Linux and Windows. The baseline had 713 passing Rust tests and one ignored timing exercise. No baseline failure was deleted.

## Guarantee register

| Consumer decision | Designated check | Independent oracle and plausible miss |
| --- | --- | --- |
| Article authors receive the correct rule or safe fix for each supported syntax and adapter policy | The named rule check below, one per distinct rule; sentence-case and malformed-heading variants are rows | Literal wikitext and adapter policy supply the expected rule, absence, or replacement. A parser or policy refactor could silently misclassify a rare input. |
| Heading lint distinguishes actual malformed headings from valid extension and template syntax | `malformed_heading_lint_distinguishes_wikitext_syntax` | The four hand-written wikitext forms include the recent trailing `=` regression and a missing opening marker. |
| Sentence-case lint preserves title-derived proper nouns without promoting ordinary words | `sentence_case_heading_preserves_local_proper_nouns` | The title and heading literals specify five distinct outcomes, including the exact suggested replacement. |
| Template readers can get a bounded text brief, a full text view, and every declared key in brief JSON | `template_show_brief_exposes_all_declared_keys_without_full_text_detail` | Thirteen source placeholders plus one TemplateData-only parameter cross the twelve-key text cap; the JSON expectation is the explicit template contract. Ignoring `--view` or capping JSON silently loses information. |
| Catalog refresh remains fast enough for users | No designated check from `catalog_refresh_timing` | The ignored exercise had no runtime threshold or scheduled gate; measure this with an owned benchmark if performance becomes a release criterion. |

## Verdicts

The following former tests were **folded**, retaining their distinct input rows:

| Former test | Designated check |
| --- | --- |
| `detects_sentence_case_heading` | `sentence_case_heading_preserves_local_proper_nouns` |
| `sentence_case_heading_exempts_proper_nouns` | `sentence_case_heading_preserves_local_proper_nouns` |
| `sentence_case_heading_preserves_proper_nouns_in_suggestion` | `sentence_case_heading_preserves_local_proper_nouns` |
| `sentence_case_heading_still_flags_non_proper_nouns` | `sentence_case_heading_preserves_local_proper_nouns` |
| `sentence_case_heading_does_not_promote_lowercase_title_words` | `sentence_case_heading_preserves_local_proper_nouns` |
| `accepts_tabber_separator_lines_as_extension_markup` | `malformed_heading_lint_distinguishes_wikitext_syntax` |
| `accepts_template_parameter_lines_that_end_with_equals` | `malformed_heading_lint_distinguishes_wikitext_syntax` |
| `accepts_template_call_lines_that_end_with_a_parameter_assignment` | `malformed_heading_lint_distinguishes_wikitext_syntax` |
| `detects_heading_lines_missing_an_opening_marker` | `malformed_heading_lint_distinguishes_wikitext_syntax` |

`catalog_refresh_timing` was **deleted**: it was ignored in every gate, printed timing without judging it, and asserted only the number of fixture files it created. No product guarantee was controlled by it.

The following existing checks are **designated** for their named article-lint guarantee. Their green baseline was observed; a deliberate code break was not run for each, so their seen-red status remains unverified. A suitable break is to suppress the named rule or safe-fix branch and confirm the test fails on the expected rule or output.

| Designated test | Consumer guarantee |
| --- | --- |
| `detects_and_repairs_mojibake_before_curly_quote_lint` | Misdecoded text is diagnosed and the safe fix restores the dash. |
| `flags_replacement_character_without_guessing_at_content` | Unknown replacement characters stay errors without speculative fixes. |
| `detects_markdown_heading_and_applies_safe_fix` | Markdown headings become wikitext headings. |
| `detects_raw_wikitext_balance_errors_inside_references` | Unclosed templates inside references are diagnosed. |
| `source_review_rule_matches_a_citation_url_without_deciding_reliability` | Source-review warnings point to the URL without automated reliability judgment. |
| `detects_invalid_extension_block_shapes` | Invalid extension blocks receive their distinct rule IDs. |
| `detects_unavailable_module_functions_from_local_lua_exports` | Calls to absent local Lua functions are diagnosed. |
| `site_specific_module_semantics_are_not_hardcoded_in_generic_lint` | Generic lint does not assume a site's module semantics. |
| `lints_state_draft_with_explicit_title_override` | Draft lint uses the selected target title and namespace. |
| `safe_fix_preserves_state_draft_title_override` | A safe fix retains the selected draft target. |
| `detects_missing_short_description` | Adapter-required short descriptions are diagnosed. |
| `inserts_missing_article_quality_banner_with_safe_fix` | Safe fix inserts the adapter's required quality banner. |
| `preserves_existing_article_quality_review_states` | Existing review states are not reset by lint or fix. |
| `detects_missing_reflist_and_applies_safe_fix` | Missing required reference renderer is safely inserted. |
| `generic_adapter_requires_inline_references_to_be_rendered` | Inline references require a renderer under generic policy. |
| `inserts_reflist_before_reference_section_trailing_categories` | Renderer insertion precedes category links. |
| `detects_citation_after_punctuation_and_applies_safe_fix` | Citation punctuation is corrected. |
| `clustered_citations_move_punctuation_before_the_whole_cluster` | Clustered citations keep their order after punctuation correction. |
| `does_not_infer_remilia_parent_group_from_creator_field` | Lint does not invent an infobox parent relationship. |
| `rejects_citation_needed_templates` | Remilia policy rejects citation-needed markers. |
| `generic_adapter_does_not_impose_citation_needed_policy` | Generic policy leaves citation-needed markers alone. |
| `detects_red_links_in_see_also` | Broken see-also links are diagnosed. |
| `detects_unavailable_templates_against_local_catalog` | Missing local templates are diagnosed. |
| `detects_unknown_parameters_for_templatedata_backed_templates` | Unknown TemplateData parameter names are diagnosed. |
| `detects_unavailable_modules_for_direct_invoke` | Missing modules are diagnosed for direct invoke. |
| `accepts_direct_invoke_for_local_module` | Present local modules are admitted. |
| `detects_invoke_when_scribunto_is_not_available` | Invoke is refused without Scribunto capability. |
| `accepts_templatestyles_for_local_asset` | Present TemplateStyles assets are admitted. |
| `detects_unavailable_templatestyles_source` | Missing TemplateStyles assets are diagnosed. |
| `detects_templatestyles_missing_src` | TemplateStyles without `src` are diagnosed. |
| `detects_unsupported_extension_tags_from_capabilities` | Unsupported extension tags are diagnosed from capabilities. |
| `detects_suspicious_html_tags_even_when_they_are_not_known_extensions` | Suspicious unknown tags are diagnosed. |

The two folded checks and the new CLI check are designated for the register rows above. In a disposable source copy, changing both produced lint rule IDs made the named positive rows fail, and emptying the brief key list made the CLI check fail against the fourteen-name literal. The fresh copy compiled its own test binaries; no mutation reached the candidate checkout. Before/after for this scope: article-lint tests 41 to 34, with 42 input cases preserved; catalog ignored tests 1 to 0, removing its one ungated timing case; template CLI tests 0 to 1, adding one fourteen-key fixture. Article-lint test source fell from 1,006 to 953 lines and catalog test source from 2,195 to 2,166 lines. The new CLI test adds its own fixture and process calls. No production test seam or test dependency was added or removed. The implementation changes are the template brief and companion-release integration, independent of test pruning.

Remaining qualification is the extracted archive on Linux and both macOS targets and deliberate breaks of the other designated checks where seen-red evidence is still missing. Windows extracted-project verification and the CI-equivalent source gates are separate results; neither implies that a public release has been published.
