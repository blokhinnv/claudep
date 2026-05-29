FROM mirror.gcr.io/node:22.22.0-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
  git \
  curl \
  ca-certificates \
&& rm -rf /var/lib/apt/lists/*

# install uv
RUN curl -LsSf https://astral.sh/uv/install.sh | sh
ENV PATH="/root/.local/bin:$PATH"

RUN npm install -g @anthropic-ai/claude-code

WORKDIR /app

CMD ["tail", "-f", "/dev/null"]