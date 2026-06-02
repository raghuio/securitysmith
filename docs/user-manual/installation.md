# Installation Guide

## System Requirements

- **OS**: Linux (Ubuntu 22.04+, Fedora 38+, Debian 12+)
- **RAM**: 4 GB minimum (8 GB recommended)
- **Disk**: 500 MB for the app + space for vault and attachments
- **Display**: 1280×720 or higher

## Install from .deb (Debian/Ubuntu)

```bash
sudo dpkg -i securitysmith_0.1.0_amd64.deb
sudo apt-get install -f
```

## Install from .AppImage

```bash
chmod +x securitysmith_0.1.0_amd64.AppImage
./securitysmith_0.1.0_amd64.AppImage
```

## First Launch

On first launch, the app creates a vault directory at `~/.local/share/securitysmith/`. No admin privileges are required after installation.

## Uninstall

```bash
sudo dpkg -r securitysmith
rm -rf ~/.local/share/securitysmith/
```
