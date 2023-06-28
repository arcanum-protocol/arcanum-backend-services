FROM denoland/deno:1.34.3
WORKDIR /app
USER deno
COPY ./src/main.ts .
RUN deno cache main.ts
RUN mkdir -p /var/tmp/log
CMD ["run", "--allow-all", "main.ts"]
