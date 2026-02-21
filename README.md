![header](https://capsule-render.vercel.app/api?type=waving&color=0:1a1b27,100:2483b6&height=200&section=header&text=ZINC%20OS&fontSize=60&fontColor=70a5fd&animation=fadeIn)

Building Zinc OS from the ground up: 32-bit, x86, with custom keyboard drivers, VGA output, and a minimal shell. A playground for OS enthusiasts and low-level coding fans.

# Installation
- Clone repository
```bash
git clone https://github.com/backenduzb/zinc.git
```
- Build os
```bash
make iso
```
- You can test this os with qemu or other virtual managers
```bash
make run
```

# Zinc OS ToDo
- [x] CPU controlling
- [x] Keyboard driver
- [x] VGA output
- [x] Create commmands 
- [ ] Disk controlling
- [ ] Timer / sleep_ms
- [ ] Command history
- [ ] Tab autocomplete