# geotool haproxy image
FROM haproxy:2.5
LABEL version="1.0.0"
LABEL author="Vadim Tambovtsev <vadtam2@gmail.com>"
EXPOSE 443
COPY haproxy.cfg /usr/local/etc/haproxy/haproxy.cfg
# mysite.pem is a join of fullchain.pem and priv.key from letsencrypt
COPY mysite.pem /usr/local/etc/haproxy/mysite.pem

