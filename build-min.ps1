# Builds gh3_mqtt.dll with aggressive min-sized-rust optimizations (build-std + immediate-abort)
$env:RUSTFLAGS="-Zunstable-options -Cpanic=immediate-abort -Zlocation-detail=none -Zfmt-debug=none -C link-arg=/DEBUG:NONE"
cargo +nightly build --release --target i686-pc-windows-msvc -Z build-std=std,panic_abort
$exitCode = $LASTEXITCODE
$env:RUSTFLAGS=""

if ($exitCode -ne 0) {
    exit $exitCode
}

Get-Item target\i686-pc-windows-msvc\release\gh3_mqtt.dll | Select-Object Name, Length
