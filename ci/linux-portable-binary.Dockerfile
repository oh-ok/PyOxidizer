# Debian Jessie.
FROM --platform=linux/x86_64 debian@sha256:32ad5050caffb2c7e969dac873bce2c370015c2256ff984b70c1c08b3a2816a0
MAINTAINER Gregory Szorc <gregory.szorc@gmail.com>

RUN groupadd -g 1000 build && \
    useradd -u 1000 -g 1000 -d /build -s /bin/bash -m build && \
    mkdir /tools && \
    chown -R build:build /build /tools

ENV HOME=/build \
    SHELL=/bin/bash \
    USER=build \
    LOGNAME=build \
    HOSTNAME=builder \
    DEBIAN_FRONTEND=noninteractive

CMD ["/bin/bash", "--login"]
WORKDIR '/build'

RUN for s in debian_jessie debian_jessie-updates debian-security_jessie/updates; do \
      echo "deb http://snapshot.debian.org/archive/${s%_*}/20230322T152120Z/ ${s#*_} main"; \
    done > /etc/apt/sources.list && \
    ( echo 'quiet "true";'; \
      echo 'APT::Get::Assume-Yes "true";'; \
      echo 'APT::Install-Recommends "false";'; \
      echo 'Acquire::Check-Valid-Until "false";'; \
      echo 'Acquire::Retries "5";'; \
    ) > /etc/apt/apt.conf.d/99builder

RUN apt-get update
RUN apt-get install --force-yes \
  ca-certificates \
  curl \
  file \
  gcc \
  gcc-multilib \
  make \
  musl-tools \
  xz-utils

# We use `curl --insecure` throughout this file. This is reasonably safe since
# we validate the SHA-256 of all downloaded files to prevent tampering.

RUN curl --insecure -L https://github.com/facebook/zstd/releases/download/v1.5.7/zstd-1.5.7.tar.gz > zstd.tar.gz && \
  echo 'eb33e51f49a15e023950cd7825ca74a4a2b43db8354825ac24fc1b7ee09e6fa3  zstd.tar.gz' | sha256sum -c - && \
  mkdir zstd/ && \
  tar -C zstd/ --strip 1 -xf zstd.tar.gz && \
  make -C zstd -j `nproc` && \
  make -C zstd -j `nproc` install prefix=/usr/local && \
  rm -rf zstd.tar.gz zstd/

RUN curl --insecure -L https://github.com/indygreg/toolchain-tools/releases/download/toolchain-bootstrap%2F20250308/llvm-20.1.0+20250308-gnu_only-x86_64-unknown-linux-gnu.tar.zst > llvm.tar.zst && \
  echo '41d3d74e21e064e2e59a4e89feca74d58a5e9e95f73877f3c9ed82ca95607b47  llvm.tar.zst' | sha256sum -c - && \
  tar -C /usr/local --strip 1 -I zstd -xf llvm.tar.zst && \
  rm llvm.tar.zst

# The binutils is Jessie is too old to link the python-build-standalone distributions
# due to a R_X86_64_REX_GOTPCRELX relocation. So install a newer binutils.
RUN curl --insecure https://ftp.gnu.org/gnu/binutils/binutils-2.36.1.tar.xz > binutils.tar.xz && \
  echo 'e81d9edf373f193af428a0f256674aea62a9d74dfe93f65192d4eae030b0f3b0  binutils.tar.xz' | sha256sum -c - && \
  tar -xf binutils.tar.xz && \
  rm binutils.tar.xz && \
  mkdir binutils-objdir && \
  cd binutils-objdir && \
  ../binutils-2.36.1/configure \
    --build=x86_64-unknown-linux-gnu \
    --prefix=/usr/local \
    --enable-plugins \
    --disable-nls \
    --with-sysroot=/ && \
  make -j `nproc` && \
  make install -j `nproc` && \
  cd .. && \
  rm -rf binutils-objdir

USER build

RUN curl --insecure https://raw.githubusercontent.com/rust-lang/rustup/ce5817a94ac372804babe32626ba7fd2d5e1b6ac/rustup-init.sh > rustup-init.sh && \
  echo 'a3cb081f88a6789d104518b30d4aa410009cd08c3822a1226991d6cf0442a0f8  rustup-init.sh' | sha256sum -c - && \
  chmod +x rustup-init.sh && \
  ./rustup-init.sh -y --default-toolchain 1.82.0 --profile minimal && \
  ~/.cargo/bin/rustup target add x86_64-unknown-linux-musl

RUN curl --insecure -L https://github.com/indygreg/python-build-standalone/releases/download/20220502/cpython-3.9.12+20220502-x86_64-unknown-linux-gnu-install_only.tar.gz > python.tar.gz && \
  echo 'ccca12f698b3b810d79c52f007078f520d588232a36bc12ede944ec3ea417816  python.tar.gz' | sha256sum -c - && \
  tar -xf python.tar.gz && \
  rm python.tar.gz && \
  echo 'export PATH="$HOME/python/bin:$PATH"' >> ~/.bashrc

# Force a snapshot of the Cargo index into the image. This should hopefully
# speed up subsequent operations needing to fetch the index.
RUN ~/.cargo/bin/cargo init cargo-fetch && \
  cd cargo-fetch && \
  echo 'pyembed = "0"' >> Cargo.toml && \
  ~/.cargo/bin/cargo update && \
  cd && \
  rm -rf cargo-fetch
