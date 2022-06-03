# xDNS
## Authority name server
### Start
```xDNS auth --dns 127.0.0.1:5555 --http 127.0.0.1:8888```

### Create zone
```curl localhost:8888/AddZone -d'{"name":"com", "ips": ["1.1.1.1"]}'```
create zone com, with name server address as 1.1.1.1

### Create RRset
```
curl localhost:8888/AddRRset -d'{"zone":"com", "rrset":["uri.com. 3600 IN URI 10 1 \"ftp://ftp.example.com/public\""]}'
curl localhost:8888/AddRRset -d'{"zone":"com", "rrset":["txt2.com. 3600 IN TXT \"algo=sha256,user=xxxx,hash=xxxxx\""]}'
curl localhost:8888/AddRRset -d'{"zone":"com", "rrset":["cert.com. 3600 IN CERT 2 77 2 KR1L0GbocaIOOim1+qdHtOSrDcOsGiI2NCcxuX2/Tqc"]}'
```

## Recursor
### Start
```xDNS recursor --dns 127.0.0.1:5555 --http 127.0.0.1:8888```

### Add forward
```curl localhost:8888/AddForward -d'{"zone":"com", "addr":"114.114.114.114:53"}'```
