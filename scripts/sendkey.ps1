# Send one or more virtual keys via keybd_event. The title polls
# GetAsyncKeyState, so global key injection works without window focus.
# Usage:  .\sendkey.ps1 -Keys "0x0D" -Hold 120
#         .\sendkey.ps1 -Keys "W,A,S,D" -Hold 250 -Gap 100
param(
    [Parameter(Mandatory)][string]$Keys,
    [int]$Hold = 120,
    [int]$Gap = 80
)

$sig = @"
using System;
using System.Runtime.InteropServices;
public static class KBInject {
    [DllImport("user32.dll", SetLastError = true)]
    public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);
    public static void Tap(byte vk, int holdMs) {
        keybd_event(vk, 0, 0, UIntPtr.Zero);
        System.Threading.Thread.Sleep(holdMs);
        keybd_event(vk, 0, 2, UIntPtr.Zero);
    }
}
"@
Add-Type -TypeDefinition $sig

$map = @{
    'ENTER'    = 0x0D; 'ESC' = 0x1B; 'TAB'   = 0x09; 'SPACE' = 0x20
    'BACKSPACE'= 0x08; 'W'   = 0x57; 'A'     = 0x41; 'S'     = 0x53
    'D'        = 0x44; 'E'   = 0x45; 'Q'     = 0x51; 'R'     = 0x52
    'F'        = 0x46; 'X'   = 0x58; 'Y'     = 0x59; 'C'     = 0x43
    'Z'        = 0x5A; 'I'   = 0x49; 'J'     = 0x4A; 'K'     = 0x4B; 'L' = 0x4C
    'UP'       = 0x26; 'DOWN'= 0x28; 'LEFT'  = 0x25; 'RIGHT' = 0x27
    'SHIFT'    = 0xA0; 'CTRL'= 0xA2; 'P'     = 0x50; 'V'    = 0x56; 'M' = 0x4D; 'G' = 0x47; 'H' = 0x48
}

foreach ($tok in $Keys -split ',') {
    $name = $tok.Trim().ToUpper()
    $vk = 0
    if ($name -match '^0X[0-9A-F]+$') { $vk = [Convert]::ToInt32($name, 16) }
    elseif ($map.ContainsKey($name)) { $vk = $map[$name] }
    else { Write-Warning "Unknown key: $tok"; continue }
    [KBInject]::Tap([byte]$vk, $Hold)
    if ($Gap -gt 0) { Start-Sleep -Milliseconds $Gap }
}
Write-Output "sent: $Keys"
