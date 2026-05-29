ARG NODE_IMAGE=node:22-slim
FROM ${NODE_IMAGE}

RUN apt-get update && apt-get install -y --no-install-recommends \
  git \
  curl \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*

RUN npm install -g @anthropic-ai/claude-code

WORKDIR /app

CMD ["tail", "-f", "/dev/null"]
