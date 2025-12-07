FROM apache/age

ENV DEBIAN_FRONTEND=noninteractive

# Install pgvector
RUN apt install postgresql-18-pgvector
#RUN apt install postgresql-server-dev-18
#RUN git clone --depth 1 https://github.com/pgvector/pgvector.git /tmp/pgvector
#RUN cd /tmp/pgvector && make && make install && rm -rf /tmp/pgvector

COPY initdb /docker-entrypoint-initdb.d/

EXPOSE 5432
