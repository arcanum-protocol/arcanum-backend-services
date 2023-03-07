FROM node:14

WORKDIR /app

COPY package.json pnpm-lock.yaml ./
RUN npm install -g pnpm
RUN pnpm install

COPY . .

CMD ["pnpm", "run", "start"]
