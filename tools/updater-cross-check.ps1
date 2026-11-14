# Vérifie `karbon-updater` pour macOS et Linux depuis Windows (`cargo clippy --target`).
#
# Sa dépendance `ring` (rustls) compile du C pour la cible : avec le clang de LLVM
# (C:\Program Files\LLVM) et quelques en-têtes système de substitution, son script de
# compilation passe. Rien n'est lié : seul le code Rust est vérifié.
#
#   powershell -File tools/updater-cross-check.ps1
param([string]$Llvm = "C:\Program Files\LLVM\bin")

$ErrorActionPreference = "Stop"
$stubs = Join-Path ([IO.Path]::GetTempPath()) "karbon-updater-stub-headers"
New-Item -ItemType Directory -Force $stubs | Out-Null
Set-Content "$stubs\assert.h" "#define assert(x) ((void)0)"
Set-Content "$stubs\TargetConditionals.h" "#define TARGET_OS_IPHONE 0`n#define TARGET_OS_OSX 1`n#define TARGET_OS_MAC 1"
Set-Content "$stubs\string.h" "typedef __SIZE_TYPE__ size_t;`nvoid *memcpy(void *, const void *, size_t);`nvoid *memmove(void *, const void *, size_t);`nvoid *memset(void *, int, size_t);`nint memcmp(const void *, const void *, size_t);"
Set-Content "$stubs\stdlib.h" "typedef __SIZE_TYPE__ size_t;`nvoid *malloc(size_t);`nvoid free(void *);`nint posix_memalign(void **, size_t, size_t);`nvoid abort(void);"
Set-Content "$stubs\errno.h" "extern int errno;"

$status = 0
$installed = rustup target list --installed
foreach ($target in "x86_64-unknown-linux-gnu", "x86_64-apple-darwin", "aarch64-apple-darwin") {
    if ($installed -notcontains $target) { Write-Host "== $target : cible absente (rustup target add $target)"; continue }
    $key = $target.Replace("-", "_")
    Set-Item "env:CC_$key" "$Llvm\clang.exe"
    Set-Item "env:CFLAGS_$key" "--target=$target -I$stubs"
    Set-Item "env:AR_$key" "$Llvm\llvm-ar.exe"
    Write-Host "== $target"
    cargo clippy -p karbon-updater --all-targets --target $target -- -D warnings
    if ($LASTEXITCODE -ne 0) { $status = $LASTEXITCODE }
}
exit $status
