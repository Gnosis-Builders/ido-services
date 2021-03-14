## IDO-services

### Pull contract information

```
cd contracts/
cargo run --bin vendor --features bin
```

### Start the api servers

Start a server by:
```
cargo run --bin orderbook
```


### Postgres

The tests that require postgres connect to the default database of locally running postgres instance on the default port. There are several ways to set up postgres:

```sh
# Docker
docker run -e POSTGRES_PASSWORD=password -e POSTGRES_USER=`whoami` -p 5432:5432 postgres

where whoami is the result from the terminal command `whoami`
# Service
sudo systemctl start postgresql.service
sudo -u postgres createuser $USER
sudo -u postgres createdb $USER

# Manual setup in local folder
mkdir postgres && cd postgres
initdb data # Arbitrary directory that stores the database
# In data/postgresql.conf set unix_socket_directories to the absolute path to an arbitrary existing
# and writable directory that postgres creates a temporary file in.
# Run postgres
postgres -D data
# In another terminal, only for first time setup
createdb -h localhost $USER

# Finally for all methods to test that the server is reachable and to set the schema for the tests.
docker build --tag ido-migrations -f docker/Dockerfile.migration .
# If you are running postgres in locally, your URL is `localhost` instead of `host.docker.internal`
docker run -ti -e FLYWAY_URL="jdbc:postgresql://host.docker.internal/?user='whoami'" -v $PWD/database/sql:/flyway/sql ido-migrations migrate
```
