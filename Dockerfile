# https://www.digitalocean.com/docs/app-platform/references/app-specification-reference/

# -- Stage 1 -- #
# Compile the app.
FROM rust:1-slim as builder
WORKDIR /app
COPY . .
RUN apt-get update 
RUN apt-get -y install pkg-config 
RUN cargo install --path . 

# -- Stage 2 -- #
# Create the final environment with the compiled binary.
FROM alpine
WORKDIR /root/
# Copy the binary from the builder stage and set it as the default command.
COPY --from=builder /usr/local/cargo/bin/rustyrails /usr/local/bin/rustyrails
CMD ["rustyrails"]