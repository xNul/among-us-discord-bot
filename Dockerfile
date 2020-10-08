FROM rust:latest as builder
WORKDIR /usr/src/among-us-discord-bot
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
COPY --from=builder /usr/local/cargo/bin/among-us-discord-bot /usr/local/bin/among-us-discord-bot
CMD ["among-us-discord-bot"]