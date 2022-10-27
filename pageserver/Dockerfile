# geotool pageserver image
FROM debian:bullseye
LABEL version="0.1.0"
LABEL author="Vadim Tambovtsev <vadtam2@gmail.com>"
WORKDIR /workdir
COPY target/release/pageserver ./pageserver-exe
COPY Rocket.toml ./Rocket.toml
COPY templates ./templates
COPY web ./web
ENTRYPOINT ["./pageserver-exe"]


