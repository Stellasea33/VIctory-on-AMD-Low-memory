# AMD显卡上的VIctory低内存优化

在 Windows 上原生运行 Xbox Series 版 GTA6 的兼容层（VIctory），针对 AMD 显卡与低内存（16 GB）机器做了一些优化。

> 本项目基于 [Belobug/VIctory-Belobug-AMD](https://github.com/Belobug/VIctory-Belobug-AMD)（AMD RX 7900 系列分支）。

---

## 硬件要求

| 项目 | 最低 | 推荐 |
|---|---|---|
| CPU | 支持 AVX2（如 Intel 12 代+ / AMD Ryzen 5000+） | — |
| 显卡 | AMD RDNA3（RX 7000 系列，20 GB 显存） | RX 7900 XT/XTX |
| 内存 | 16 GB | **32 GB+** |
| 系统 | Windows 10/11| — |
| **Resizable BAR** | 建议开启（BIOS） | — |

> ⚠️ **16 GB 内存是硬瓶颈**：关 ReBAR 会 OOM（约 3 分钟崩），开 ReBAR 借显存补内存能稳定跑 20 分钟以上。想流畅游玩仍建议 32 GB+。

---

## 前置条件

### 1. PC 版 FSR DLL（仅 2K/4K 输出需要）
2K/4K 输出使用宿主的 PC 版 FSR 升采样器（`amd_fidelityfx_upscaler_dx12.dll`）。

**本包已自带该 DLL**：为避开 GitHub 单文件上传限制，升采样器以压缩形式存放（`fidelityfx/amd_fidelityfx_upscaler_dx12.dll.z`），`cargo build` 编译时会自动解压出 `amd_fidelityfx_upscaler_dx12.dll`（见 `apps/victory/build.rs`）。`victory.toml` 已预配置 `fidelityfx_dir = 'fidelityfx'`，编译后即可直接用，无需额外下载。

### 2. 着色器预热（强烈建议）
首次运行前预热全部着色器，避免进世界时边玩边编译：

```powershell
.\target\diag\victory.exe shaders
```

约 2.5 分钟，完成后着色器缓存命中率 100%（日志 `misses=0`）。

---

## 编译

需要 Rust 工具链 + MSVC 构建工具（Visual Studio Build Tools）：

```powershell
cargo build --profile diag -p victory --features diagnostics
```

产物：`target\diag\victory.exe`

---

## 配置（victory.toml）

```toml
[paths]
game_dir = 'D:\GTA6'                                        # 你的游戏目录
fidelityfx_dir = 'fidelityfx'                               # PC FSR DLL 目录（本包已自带）
```

---

## 启动方式

所有方式都在 PowerShell 里运行，均自动启用内存优化（residency 准入控制 + ReBAR 显存卸载 + OOM 重试 + 空页/断点容错）。

### 方式一：1K 输出（1920×1080）—— 最稳定，推荐

```powershell
.\scripts\play-lowmem.ps1 -Exe "D:\GTA6\game_scarlett_master.exe" -ConsoleProfile series-s -Upscaler 6
```

| 项目 | 说明 |
|---|---|
| 档位 | Series S（标题内存约 8 GB） |
| 升采样 | 游戏内置（`-Upscaler 6`，避免 FSR 的横线伪影） |
| 帧率 | 19 ~ 29 fps（最流畅） |
| 内存压力 | 最低 |

### 方式二：2K 输出（2560×1440）—— 画面与帧率平衡

```powershell
.\scripts\play-lowmem.ps1 -Exe "D:\GTA6\game_scarlett_master.exe" -ConsoleProfile series-x -More @{VICTORY_FSR_4K='balanced'; VICTORY_OUTPUT='2560x1440'}
```
如果想尝试4k也可以改为3840x2160

## 代码优化说明

| 文件 | 改动 |
|---|---|
| `crates/victory-loader/src/null_page.rs`（新增） | 空页写 + int3 断点的 VEH 处理器，修复 0xc0000005 崩溃 |
| `crates/victory-gpu/src/host.rs` | 上传/共享缓冲 OOM 重试；着色器输入放显存（`local_gpu_inputs`，移植自 DexNom） |
| `crates/victory-gpu/src/descriptor/host.rs` | 着色器输入分配点路由到显存 |
| `apps/victory/src/run.rs` | 安装空页处理器；FSR 4K 方案支持自定义输出分辨率 |
| `crates/victory-loader/src/lib.rs` | 导出空页处理器 |
| `scripts/play-lowmem.ps1`（新增） | 低内存启动脚本（residency + ReBAR 卸载 + 分辨率/升采样参数） |

---

## 已知问题

- **树木/植被闪烁**：渲染正确性问题（culling/LOD 着色器翻译精度），属上游"修复渲染"范畴，非本包内存优化能解决。
- **FSR 升采样有横线伪影**：FSR 3.1 时间性升采样的固有现象，用游戏内置升采样器（方式一）可避免。

## 免责声明

本包仅供学习研究使用。请使用你自己合法拥有的游戏副本。
