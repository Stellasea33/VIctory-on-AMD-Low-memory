# Low-memory launcher: caps VIctory's host-memory usage for machines with
# 16 GB of RAM or less.
#   scripts\play-lowmem.ps1 -Exe "D:\GTA6\game_scarlett_master.exe"
# -ConsoleProfile series-s  Series S profile (default: title memory drops from
#                           24 GB to ~8 GB, the biggest memory saving)
#                           series-x  Series X profile (better picture, more RAM)
# -NoDepthTwins   VICTORY_DEPTH_TWINS=0 (halves depth-texture twins; unordered
#                 access views of depth textures become null, may show artifacts)
# -Safe           without the newer, faster ways (try first when something is wrong)
# -WindowsBarMB N cap (MiB) of windows placed in VRAM (VICTORY_WINDOWS_BAR_MB).
#                 Default 4096. Resizable BAR on can hang the GPU (driver TDR,
#                 DXGI_ERROR_DEVICE_HUNG); lower it (2048/1024) or 0 to trade
#                 memory back for stability.
# -FfxScale S     FidelityFX render scale 0.5..1.0 (VICTORY_FFX_SCALE). Series S
#                 renders at 2/3 and upscales to the output, which shows
#                 intermittent horizontal lines; 1.0 renders native (no upscale,
#                 cleaner picture, heavier on the GPU, lower fps).
# -Upscaler K     VICTORY_UPSCALER: 7 = FidelityFX (Series S default), 6 = the
#                 title's own upscaler (Series X default).
# Unlike play.ps1, this also turns on VICTORY_RESIDENCY=1 (host-memory admission
# control, ported from the cool_fox fork, commit 706305b): under memory pressure
# it limits texture prefetch, frees idle scratch/staging blocks and stops the
# texture pass before commit runs out, avoiding the 0x8007000E crashes.
param(
    [Parameter(Mandatory)][string]$Exe,
    [ValidateSet('series-x', 'series-s')][string]$ConsoleProfile = 'series-s',
    [switch]$NoDepthTwins,
    [switch]$Safe,
    [switch]$RayTracing,
    [int]$WindowsBarMB = 4096,
    [string]$FfxScale,
    [ValidateSet('6', '7')][string]$Upscaler,
    [ValidateSet('diag', 'release')][string]$Build = 'diag',
    [hashtable]$More = @{}
)

$settings = @{
    VICTORY_TEXTURE_NO_WAIT = '1'; VICTORY_RELAY = '2'; VICTORY_WIDE_BACK = '1'; VICTORY_CARD_REHASH_RUNS = '1'
    VICTORY_JOIN_COPIES = '1'; VICTORY_INDIRECT_WINDOW = '1'; VICTORY_RELAY_COPY = '1'; VICTORY_CARD_STEPS = '2'
    VICTORY_INDIRECT_BATCH = '2'; VICTORY_MAP_THREADS = '1'; VICTORY_TITLE_PRIORITIES = '0'; VICTORY_CARD_AWAKE = '1'
    VICTORY_SHOW_AFTER = '1'; VICTORY_AMBIENT = '0.8'
    # Memory optimization (added vs play.ps1):
    VICTORY_RESIDENCY = '1'           # host-memory admission control (cool_fox)
    VICTORY_RESIDENCY_LIMITED = '1'   # force the smaller-PC budgets
    VICTORY_LOCAL_GPU_INPUTS = '1'    # shader inputs in VRAM when the budget allows (DexNom)
    # Windows (vertex/index/constant buffers shaders read) in VRAM with no CPU
    # side; needs Resizable BAR (inactive without it). ~3.5 GiB of host RAM stays
    # free, but too much can hang the GPU (driver TDR): tune via -WindowsBarMB.
    VICTORY_WINDOWS_BAR_MB = "$WindowsBarMB"
}
if ($FfxScale) { $settings.VICTORY_FFX_SCALE = $FfxScale }
if ($Upscaler) { $settings.VICTORY_UPSCALER = $Upscaler }
if ($Build -eq 'diag') { $settings.VICTORY_D3D12_DRED = 'light' }
if ($Safe) {
    $settings.VICTORY_UNDER_WIDE = '0'; $settings.VICTORY_LAND_ECHO = '0'
} else {
    $settings.VICTORY_THIN = '1'; $settings.VICTORY_JOIN_PARTS = '8'; $settings.VICTORY_UPLOAD_AHEAD = '1'
}
if ($RayTracing) { $settings.VICTORY_BVH_BUILDER = '1' }
if ($NoDepthTwins) { $settings.VICTORY_DEPTH_TWINS = '0' }
foreach ($k in $More.Keys) { $settings[$k] = $More[$k] }

$victory = Join-Path $PSScriptRoot "..\target\$Build\victory.exe"
if (-not (Test-Path $victory)) { Write-Host "No $victory. Build it first (README.md)."; exit 1 }
if (-not (Test-Path $Exe)) { Write-Host "No $Exe."; exit 1 }

$paths = 'VICTORY_CONFIG', 'VICTORY_GAME_DIR', 'VICTORY_CACHE_DIR', 'VICTORY_DATA_DIR', 'VICTORY_FIDELITYFX_DIR'
$inherited = @(Get-ChildItem Env:VICTORY_* | Where-Object { $_.Name -notin $paths })
foreach ($entry in $inherited) { Remove-Item "Env:$($entry.Name)" }
foreach ($k in $settings.Keys) { Set-Item "Env:$k" $settings[$k] }
$discarded = $inherited | Where-Object {
    $_.Name -notin $More.Keys -and -not $_.Value.Equals([Environment]::GetEnvironmentVariable($_.Name))
} | Sort-Object Name
if ($discarded) {
    Write-Host "Discarded inherited values for $($discarded.Name -join ', '). Use -More to pass them explicitly."
}

$Exe = (Resolve-Path $Exe).Path
Push-Location (Join-Path $PSScriptRoot '..')
try { & $victory run $Exe --execute --console $ConsoleProfile } finally { Pop-Location }
