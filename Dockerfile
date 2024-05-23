# Nix builder
FROM nixpkgs/nix-flakes:latest AS builder

COPY . /tmp/build
WORKDIR /tmp/build

RUN nix build

RUN mkdir /tmp/nix-store-closure
RUN cp -R $(nix-store -qR result/) /tmp/nix-store-closure

FROM scratch

WORKDIR /app

COPY --from=builder /tmp/nix-store-closure /nix/store
COPY --from=builder /tmp/build/result /app
CMD ["/app/bin/shoelace"]
