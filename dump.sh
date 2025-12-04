#!/bin/sh

host="$(dirname $0)"
dev_env="$host/.env"
config="$host/dump_config.toml"

if [ -f "$dev_env" ]; then
  source "$dev_env"
else
  echo "dev compose .env not found"
  exit 1
fi


diesel print-schema --config-file "$config" --database-url "postgres://$DB_USER:$DB_PASS@$DB_HOST/$DB_NAME" > "$host/src/schema.rs"