

No additional tests are needed for this change. The 4-line wiring-point modification (`PathBuf::join(instance)`) is already exercised by ~14 integration tests that run the full pipeline with instance-scoped paths. Adding tests would either assert standard library behavior or snapshot static config values, neither of which adds meaningful coverage.