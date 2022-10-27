# geotool database image
FROM postgres:14.3
LABEL version="1.0.0"
LABEL author="Vadim Tambovtsev <vadtam2@gmail.com>"
ENV POSTGRES_PASSWORD=my-secret-pw
ENV POSTGRES_USER=pageserver
ENV POSTGRES_DB=geotool
EXPOSE 5432

COPY setup.sql /docker-entrypoint-initdb.d
COPY postgresql.conf /var/lib/postgresql/data/postgresql.conf


