FROM ubuntu:latest as builder-stage

# Set the working directory
WORKDIR /root

# Install necessary tools to build RISC-V tests
RUN apt-get update && apt-get install -y curl git autoconf g++ build-essential

# Download RISC-V GNU toolchain binaries
RUN curl -fsSL https://github.com/riscv-collab/riscv-gnu-toolchain/releases/download/2023.05.19/riscv32-elf-ubuntu-22.04-nightly-2023.05.19-nightly.tar.gz -o out.tar.gz
RUN tar -xvf out.tar.gz

# Set the environment variables
ENV PATH="/root/riscv/bin:${PATH}"
ENV RISCV="/root"

# Clone the RISC-V tests repository
RUN git clone https://github.com/riscv/riscv-tests.git

# Set the environment variable for the RISC-V tests
ENV RISCV_TEST="/root/riscv-tests/isa"

# Update tests
RUN cd riscv-tests && \
    git submodule update --init --recursive

# Edit env - the starting address is 0x8000_0000 (default), but we want something smaller.
# Lets use 0x0700_0000. This is used in compiling the ELF binaries in the next step.
RUN sed -i "s|0x80000000|0x07000000|g" /root/riscv-tests/env/p/link.ld

# Build the tests - we are interested in rv32ui and rv32um
RUN cd riscv-tests && autoconf && \
    ./configure --prefix=/root/riscv-tests && \
    cd isa && \
    make rv32ui XLEN=32 && \
    make rv32um XLEN=32

FROM scratch as exporter-stage
COPY --from=builder-stage /root/riscv-tests/isa /
