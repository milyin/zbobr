## Verification steps

1. Run `cargo build -p zbobr` to confirm it compiles
2. Run `zbobr --help` and verify `--logs` appears in the output
3. Run a command without `--logs` and confirm no log output appears on stdout/stderr
4. Run with `--logs` and confirm info-level logs appear
5. Run with `--logs` and `RUST_LOG=debug` and confirm debug-level logs appear