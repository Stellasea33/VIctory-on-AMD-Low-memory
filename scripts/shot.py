import sys
import ctypes
from ctypes import wintypes
import mss

user32 = ctypes.windll.user32

def find_window():
    # Find the victory window by title
    hwnd = None
    @ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    def cb(h, l):
        nonlocal hwnd
        title = ctypes.create_unicode_buffer(256)
        user32.GetWindowTextW(h, title, 256)
        if 'VIctory' in title.value or 'Americas' in title.value:
            hwnd = h
            return False
        return True
    user32.EnumWindows(cb, 0)
    return hwnd

hwnd = find_window()
if not hwnd:
    print("NO_WINDOW")
    sys.exit(1)

# Bring the game window to the foreground so it is not covered.
user32.ShowWindow(hwnd, 9)  # SW_RESTORE
user32.SetForegroundWindow(hwnd)
import time
time.sleep(0.6)

rect = wintypes.RECT()
user32.GetWindowRect(hwnd, ctypes.byref(rect))
left, top, right, bottom = rect.left, rect.top, rect.right, rect.bottom
w, h = right - left, bottom - top
print(f"window {left},{top} {w}x{h}")

out = sys.argv[1] if len(sys.argv) > 1 else "shot.png"
with mss.mss() as sct:
    mon = {"left": left, "top": top, "width": w, "height": h}
    img = sct.grab(mon)
    mss.tools.to_png(img.rgb, img.size, output=out)
print(f"saved {out}")
