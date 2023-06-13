FROM node:19

WORKDIR /app

RUN npm install -g pnpm

COPY . .
RUN pnpm install
RUN pnpm build

CMD ["pnpm", "run", "run"]
