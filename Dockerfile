ARG DOCKER_NAMESPACE=omnectweucopsacr.azurecr.io
ARG BUILD_IMAGE=${DOCKER_NAMESPACE}/rust:bookworm
ARG DISTROLESS_IMAGE=gcr.io/distroless/base-debian12:nonroot

FROM ${DISTROLESS_IMAGE} AS distroless

FROM ${DOCKER_NAMESPACE}/rust:bookworm AS builder

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    cmake \
    libcurl4 \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

ARG TARGETARCH
WORKDIR "/work"

COPY --from=distroless /var/lib/dpkg/status.d /distroless_pkgs

COPY Cargo.lock Cargo.toml ./
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry,id=cargo-registry-${TARGETARCH} \
    --mount=type=cache,target=/work/build,id=cargo-build-${TARGETARCH} \
    cargo auditable build --profile dist -p windfarm-monitoring --target-dir ./build && \
    cp ./build/dist/windfarm-monitoring /work/windfarm-monitoring-bin

SHELL ["/bin/bash", "-c"]
RUN <<EOT
    set -eu

    mkdir -p /copy/status.d

    executable=(windfarm-monitoring-bin)

    mkdir -p /copy/$(dirname "${executable}")
    cp "${executable}" /copy/"${executable}"

    # gather libraries installed in distroless image to skip them
    readarray -t FILTER < <(for file in $(find /distroless_pkgs -type f -! -name "*.md5sums"); do sed -n "s/Package: \(.*\)$/\1/p" $file; done)

    # skip .so of the dynamic linker
    LOADER=$(readelf -l "${executable}" | grep "interpreter:" | sed -e "s/.*interpreter: \(.*\)]$/\1/")

    readarray -t LIBS < <(ldd "${executable}" | awk '{if ($3 == "") print $1; else print $3}')

    for LIB in ${LIBS[@]}; do
        # skip the linker loader
        if [ "$LIB" == "$LOADER" ]; then
            continue
        fi

        # the actual library location in the package may deviate from what the
        # linker specifies, so update that info and gather the package name.
        PKG_INFO=$(LOCALE=C.UTF-8 dpkg -S "*$LIB" 2> /dev/null) || continue
        PKG="${PKG_INFO%%:*}"
        LIB="${PKG_INFO##*: }"

        # skip libraries already installed in distroless
        if [[ " ${FILTER[*]} " =~ "${PKG} " ]]; then
            continue
        fi

        # copy the library and its dpkg database entries
        mkdir -p /copy/$(dirname "${LIB}")
        cp "${LIB}" /copy/"${LIB}"
        sed -n "/Package: ${PKG}/,/^$/p" /var/lib/dpkg/status > "/copy/status.d/${PKG}"
    done
EOT

FROM ${DISTROLESS_IMAGE} AS base
COPY --from=builder /work/windfarm-monitoring-bin /windfarm-monitoring
COPY --from=builder /copy/lib/ /lib/
COPY --from=builder /copy/usr/ /usr/
COPY --from=builder /copy/status.d /var/lib/dpkg/status.d

WORKDIR "/"

ENTRYPOINT [ "/windfarm-monitoring" ]
