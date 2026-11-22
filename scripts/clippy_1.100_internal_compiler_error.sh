cargo clean
git checkout cargo_clippy_1.100_internal_compiler_error~
cargo clippy -j16 -- -A clippy::redundant_field_names # Works
git checkout cargo_clippy_1.100_internal_compiler_error
cargo clippy -j16 -- -A clippy::redundant_field_names # Triggers ICE
