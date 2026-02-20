FROM archlinux:latest

RUN pacman -Syu --noconfirm && \
    pacman -S --noconfirm \
        nasm \
        gcc \
        binutils \
        grub \
        xorriso \
        make \
        cmake

WORKDIR /workspace
CMD ["bash"]