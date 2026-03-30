FROM node:22-slim

WORKDIR /app

# Copy runtime
COPY runtime/package.json runtime/package-lock.json runtime/
RUN cd runtime && npm ci --production=false

COPY runtime/tsconfig.json runtime/
COPY runtime/src/ runtime/src/

# Copy compiler
COPY compiler/ts/package.json compiler/ts/package-lock.json compiler/ts/
RUN cd compiler/ts && npm ci --production=false

COPY compiler/ts/tsconfig.json compiler/ts/
COPY compiler/ts/src/ compiler/ts/src/

# Install tsx globally (used by japl run)
RUN npm install -g tsx

# Build both
RUN cd runtime && npx tsc || true
RUN cd compiler/ts && npx tsc || true

# Copy test programs
COPY test/distributed/programs/ programs/

# Default: run a JAPL program
ENTRYPOINT ["node", "compiler/ts/dist/index.js", "run"]
