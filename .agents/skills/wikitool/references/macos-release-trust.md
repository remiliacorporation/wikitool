## macOS release trust

Unsigned macOS release executables may carry `com.apple.quarantine` after a browser download,
and Gatekeeper may block first launch (`Killed: 9` or a warning dialog); Wikitool
cannot clear this itself. From the project root, check read-only with `xattr
tools/wikitool/bin/wikitool tools/contextmink/bin/contextmink tools/papertiger/bin/papertiger`.
If any line names `com.apple.quarantine`, explain plainly that macOS flagged these downloaded
tools as unverified, that one command clears exactly the listed files without changing
Gatekeeper for anything else, and that each update needs it again. With the user's approval,
run one `xattr -d com.apple.quarantine` command naming only the flagged files by absolute path.
Add `tools/papertiger/bin/papertiger-mise` only when a task uses it. Never use `xattr -r`,
`xattr -c`, `sudo`, `spctl` or a directory argument, and do not retry a declined request.
