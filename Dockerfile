FROM alpine:3.19

ARG TARGETARCH

WORKDIR /app

# 安装 CA 证书（HTTPS 请求需要）
RUN apk add --no-cache ca-certificates tzdata

RUN mkdir -p /app/data && chmod 755 /app/data

# 根据目标架构自动选择二进制
COPY helm-linux-${TARGETARCH}-bin /app/helm
RUN chmod +x /app/helm

# 声明挂载点（数据和配置目录）
VOLUME ["/app/data"]

# 默认端口
EXPOSE 3088

ENTRYPOINT ["/app/helm"]
