FROM python:3.12-bullseye AS builder

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# 设置国内 apt 源（可选，Debian 使用阿里云或清华源）
# 更新源
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl build-essential libopus-dev \
    && rm -rf /var/lib/apt/lists/*

# 设置 Rust 国内镜像并安装 Rust
ENV RUSTUP_DIST_SERVER=https://mirrors.ustc.edu.cn/rust-static
ENV RUSTUP_UPDATE_ROOT=https://mirrors.ustc.edu.cn/rust-static/rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# 安装 uv 工具
COPY --from=ghcr.io/astral-sh/uv:0.7 /uv /uvx /bin/

# 激活 Rust 环境
ENV PATH="$HOME/.cargo/bin:$PATH"

# 设置工作目录
WORKDIR /app

# 复制整个当前目录（所有代码和配置文件）
COPY . .
COPY client-rust ./client-rust

# 修改 Cargo.toml 路径并编译（支持缓存）
RUN sed -i 's|\.\./\.\./packages/||g' Cargo.toml && \
    cargo build --release

# 设置 PyPI 加速源（提前设置）
ENV UV_PYPI_INDEX_URL=https://pypi.tuna.tsinghua.edu.cn/simple

# 安装 Python 依赖
RUN --mount=type=cache,target=/root/.cache/uv \
    uv remove pyaudio && uv sync --locked --no-install-project --no-editable

# 构建 wheel 并移除 maturin
RUN uv run maturin build --release && uv remove maturin


# ============
# 最终运行镜像
# ============
FROM python:3.12-slim

# 更新源 & 安装依赖
# 更新源
RUN apt-get update && apt-get install -y --no-install-recommends \
    libopus-dev \
    && rm -rf /var/lib/apt/lists/*

ENV CLI=true
WORKDIR /app

# 复制构建产物
COPY --from=builder /app/.venv /app/.venv
COPY main.py .
COPY xiaozhi ./xiaozhi


# 启动
CMD ["/bin/bash", "-c", "source /app/.venv/bin/activate && python xiaozhi/services/audio/kws/keywords.py && python main.py"]
