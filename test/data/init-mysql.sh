#! /bin/sh
#

MP="-h 127.0.0.1 -u root"

mysql $MP << __EOF__
DROP DATABASE IF EXISTS paf_choix;
DROP USER IF EXISTS paf_user;
CREATE DATABASE paf_choix;
CREATE USER paf_user IDENTIFIED BY 'paf_user';
GRANT ALL ON paf_choix.* TO paf_user;
CONNECT paf_choix;
$(cat mysql.dump)
__EOF__
