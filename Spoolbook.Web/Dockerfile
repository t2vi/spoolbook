# Build context is the repo root (docker-compose.yml sets it) since this needs both
# Spoolbook.Web/Spoolbook.Desktop (.NET) and spoolbook-web-svelte (the frontend) as siblings.

FROM node:22-slim AS frontend-build
WORKDIR /src/spoolbook-web-svelte
COPY spoolbook-web-svelte/package.json spoolbook-web-svelte/package-lock.json ./
RUN npm ci
COPY spoolbook-web-svelte/ ./
RUN npm run build

FROM mcr.microsoft.com/dotnet/sdk:10.0 AS backend-build
WORKDIR /src
COPY Spoolbook.Desktop/ Spoolbook.Desktop/
COPY Spoolbook.Web/ Spoolbook.Web/
RUN dotnet publish Spoolbook.Web/Spoolbook.Web.csproj -c Release -o /app/publish

FROM mcr.microsoft.com/dotnet/aspnet:10.0
WORKDIR /app
COPY --from=backend-build /app/publish .
COPY --from=frontend-build /src/spoolbook-web-svelte/build ./svelte-build

ENV SPOOLBOOK_STATIC_ROOT=/app/svelte-build
ENV SPOOLBOOK_DB_PATH=/data/spoolbook.db
ENV ASPNETCORE_URLS=http://0.0.0.0:5070
VOLUME /data
EXPOSE 5070

ENTRYPOINT ["dotnet", "Spoolbook.Web.dll"]
