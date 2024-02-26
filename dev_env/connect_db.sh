#!/bin/sh
mysql -u root --password={{mysql_password}} -h 127.0.0.1 -P 3306 {{mysql_db_name}}
