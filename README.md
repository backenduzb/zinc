![header](https://capsule-render.vercel.app/api?type=waving&color=0:1a1b27,100:2483b6&height=200&section=header&text=ZINC%20OS&fontSize=60&fontColor=70a5fd&animation=fadeIn)

We have simple tty

# Installation
- Clone repository
```bash
git clone https://github.com/backenduzb/zinc.git
```
- Build os
```bash
make -C boot/ap 
make install-splash
make
```
- You can test this os with qemu or other virtual managers
```bash
make run
```

# Zinc OS ToDo
- [x] CPU controlling
- [x] Keyboard driver
- [x] Framebuffer output
- [x] Create commmands 
- [x] Disk controlling
- [x] Timer / sleep_ms
- [x] Threads
- [x] ELF loader
- [ ] Command history
