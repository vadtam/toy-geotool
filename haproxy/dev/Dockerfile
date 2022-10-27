# geotool haproxy image
FROM haproxy:2.5
LABEL version="1.0.0"
LABEL author="Vadim Tambovtsev <vadtam2@gmail.com>"
EXPOSE 8001
COPY haproxy.cfg /usr/local/etc/haproxy/haproxy.cfg
COPY mysite.pem /usr/local/etc/haproxy/mysite.pem


