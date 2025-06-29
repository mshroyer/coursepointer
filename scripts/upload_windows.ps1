cargo build -r
Compress-Archive -Path target/release/coursepointer.exe,docs/third_party_licenses.md -Destination coursepointer-windows.zip
python3 scripts/release.py upload coursepointer-macos.zip
