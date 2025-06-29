param([string]$version)

cargo build -r
cp docs/bdist_readme.txt README.txt
Compress-Archive -Path target/release/coursepointer.exe,README.txt,LICENSE.txt,docs/third_party_licenses.md -Destination coursepointer-windows-v$version.zip
python3 scripts/release.py upload coursepointer-windows-v$version.zip
