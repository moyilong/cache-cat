FROM alpine

ARG TARGETPLATFORM

RUN apk add --no-cache ca-certificates redis

COPY --chmod=0777 ./dist/${TARGETPLATFORM} /cache_cat

# Port of Redis
EXPOSE 6379 

# Port of raft
EXPOSE 5001

# Health Check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD redis-cli ping | grep -q PONG || exit 1


ENTRYPOINT ["/cache_cat","--advertise-host","0.0.0.0]
