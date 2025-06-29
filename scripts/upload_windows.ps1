$ErrorActionPreference = 'Stop'
$version = python3 scripts/release.py head
if ($LASTEXITCODE -ne 0) {
  throw "release.py head failed with exit code $LASTEXITCODE"
}

cargo build -r
if ($LASTEXITCODE -ne 0) {
  throw "cargo build failed with exit code $LASTEXITCODE"
}

cp docs/bdist_readme.txt README.txt
Compress-Archive -Path target/release/coursepointer.exe,README.txt,LICENSE.txt,docs/third_party_licenses.md `
    -Destination coursepointer-windows-v${version}.zip

python3 scripts/release.py upload coursepointer-windows-v$version.zip
if ($LASTEXITCODE -ne 0) {
  throw "release.py upload failed with exit code $LASTEXITCODE"
}
