FROM denoland/deno:1.17.1
WORKDIR /app
USER deno
COPY ./src/main.ts .
RUN deno cache main.ts
RUN mkdir -p /var/tmp/log
CMD ["run", "--allow-all", "main.ts"]
