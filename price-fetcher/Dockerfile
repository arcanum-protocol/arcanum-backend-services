FROM node:20

WORKDIR /app

RUN npm install -g pnpm

COPY . .
RUN pnpm install
RUN pnpm build

CMD ["pnpm", "run", "run"]
