# https://www.digitalocean.com/docs/app-platform/references/app-specification-reference/

# -- Stage 1 -- #
# Compile the app.
FROM rust:1-bookworm as builder
RUN apt-get update 
RUN apt-get -y install libssl3  pkg-config 
#RUN ln -s libssl.so.3 libssl.so
#RUN ldconfig 
#RUN apk update
#RUN apk add openssl-dev musl-dev gcc
WORKDIR /app
COPY . .
RUN cargo install --path . 

# -- Stage 2 -- #
# Create the final environment with the compiled binary.
FROM debian:bookworm-slim
WORKDIR /root/
# Copy the binary from the builder stage and set it as the default command.
COPY --from=builder /usr/local/cargo/bin/rustyrails /usr/local/bin/
#RUN ls
#RUN ls /usr/local/bin
#RUN echo $PATH
RUN chmod +x /usr/local/bin/rustyrails
CMD ["/usr/local/bin/rustyrails","serve","--autofetch"]
# CMD ["ls","-al","/usr/local/bin/rustyrails"]
#CMD ["/usr/local/bin/rustyrails"]
# CMD ["file","/usr/local/bin/rustyrails"]
# ENTRYPOINT [ "/usr/local/bin/rustyrails" ] 
# serve --autofetch