## macOS release trust

First verify the archive against the release's external `SHA256SUMS.txt`, then inspect
`tools/wikitool/macos-release-trust.json`, which must identify an `unsigned_github_release`. Explain that Wikitool
cannot repair quarantine before its first execution and that the checksum does not provide an
Apple identity. After the user approves those exact verified bytes, use `xattr -d
com.apple.quarantine` only on each exact executable path they intend to run: `tools/wikitool/bin/wikitool`,
`tools/contextmink/bin/contextmink`, `tools/papertiger/bin/papertiger`, and, only if requested,
`tools/papertiger/bin/papertiger-mise`. Never use recursive quarantine removal on a download directory or
disable Gatekeeper globally.
