FROM scratch

ARG TARGETPLATFORM

COPY ./dist-docker/${TARGETPLATFORM}/cache_cat /cache_cat

ENTRYPOINT ["/cache_cat"]