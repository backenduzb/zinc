FROM archlinux:latest

RUN pacman -Syu --noconfirm && \
    pacman -S --noconfirm \
        nasm \
        gcc \
        binutils \
        grub \
        xorriso \
        make \
        cmake \
        qemu \

WORKDIR /workspace
CMD ["bash"]