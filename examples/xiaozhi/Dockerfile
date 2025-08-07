FROM python:3.12-bullseye AS builder

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# 设置国内 apt 源（可选，Debian 使用阿里云或清华源）
RUN sed -i 's|http://deb.debian.org|https://mirrors.tuna.tsinghua.edu.cn|g' /etc/apt/sources.list
RUN apt-get update
RUN apt-get install -y --no-install-recommends curl build-essential libopus-dev
RUN rm -rf /var/lib/apt/lists/*

# 设置 Rust 国内镜像并安装 Rust
ENV RUSTUP_DIST_SERVER=https://mirrors.ustc.edu.cn/rust-static
ENV RUSTUP_UPDATE_ROOT=https://mirrors.ustc.edu.cn/rust-static/rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# 激活 Rust 环境，确保 cargo 可用
ENV PATH="/root/.cargo/bin:${PATH}"

# 安装 uv 工具
COPY --from=ghcr.io/astral-sh/uv:0.7 /uv /uvx /bin/

# 设置工作目录
WORKDIR /app

# 复制 Rust 构建相关文件，利用缓存
COPY client-rust/Cargo.toml ./client-rust/Cargo.toml
COPY client-rust/Cargo.lock ./client-rust/Cargo.lock
COPY Cargo.toml Cargo.lock ./

# 创建一个空的 main.rs 避免 cargo build 报错
RUN mkdir -p client-rust/src && echo "fn main() {}" > client-rust/src/main.rs

# 预构建 client-rust 依赖
RUN cd client-rust && cargo build --release

# 复制真实 Rust 代码
COPY client-rust/src ./client-rust/src

# 重新编译 Rust 业务代码
RUN cd client-rust && cargo build --release

# **核心改动**：先复制 pyproject.toml，保证 uv 依赖安装命令能找到它
COPY pyproject.toml ./

# 设置 PyPI 加速源和超时
ENV UV_PYPI_INDEX_URL=https://pypi.tuna.tsinghua.edu.cn/simple
ENV UV_HTTP_TIMEOUT=300

# 复制其他项目文件（Python代码等）
COPY . .

# 安装 Python 依赖
RUN --mount=type=cache,target=/root/.cache/uv \
    uv remove pyaudio && uv sync --locked --no-install-project --no-editable

# 构建 Python 扩展模块
RUN uv run maturin build --release && uv remove maturin


# =============
# 最终运行镜像
# =============
FROM python:3.12-bullseye

RUN sed -i 's|http://deb.debian.org|https://mirrors.tuna.tsinghua.edu.cn|g' /etc/apt/sources.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends libopus-dev \
    && rm -rf /var/lib/apt/lists/*

ENV CLI=true
WORKDIR /app

COPY --from=builder /app/.venv /app/.venv
COPY --from=builder /app .

CMD ["/bin/bash", "-c", "source /app/.venv/bin/activate && python xiaozhi/services/audio/kws/keywords.py && python main.py"]
