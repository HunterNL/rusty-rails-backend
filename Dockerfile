# https://www.digitalocean.com/docs/app-platform/references/app-specification-reference/

# -- Stage 1 -- #
# Compile the app.
FROM rust:1-alpine as builder
#RUN apt-get update 
#RUN apt-get -y install libssl-dev pkg-config file
RUN apk update
RUN apk add openssl-dev
WORKDIR /app
COPY . .
RUN cargo install --path . 

# -- Stage 2 -- #
# Create the final environment with the compiled binary.
FROM alpine
WORKDIR /root/
# Copy the binary from the builder stage and set it as the default command.
COPY --from=builder /usr/local/cargo/bin/rustyrails /usr/local/bin/
RUN ls
RUN ls /usr/local/bin
RUN echo $PATH
RUN chmod +x /usr/local/bin/rustyrails
CMD ["/usr/local/bin/rustyrails","serve","--autofetch"]
# CMD ["ls","-al","/usr/local/bin/rustyrails"]
#CMD ["/usr/local/bin/rustyrails"]
# CMD ["file","/usr/local/bin/rustyrails"]
# ENTRYPOINT [ "/usr/local/bin/rustyrails" ] 
# serve --autofetch