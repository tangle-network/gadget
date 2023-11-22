FROM rust:latest

WORKDIR /usr/src/client
COPY . .
RUN apt-get update && apt-get install -y protobuf-compiler netcat-traditional
ENTRYPOINT ["./docker/client/client.sh"]