# syntax=docker/dockerfile:1

FROM node:22-bookworm-slim AS builder

WORKDIR /app

ARG VITE_API_BASE_URL=
ENV VITE_API_BASE_URL=$VITE_API_BASE_URL

COPY package.json package-lock.json ./
COPY apps/web/package.json apps/web/package.json
RUN npm ci

COPY apps/web apps/web
RUN npm --workspace apps/web run build

FROM caddy:2.8-alpine AS runtime

COPY docker/Caddyfile /etc/caddy/Caddyfile
COPY --from=builder /app/apps/web/build /srv

EXPOSE 80
