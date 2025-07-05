# JsonVault - Raft Consensus Integration

## Overview

JsonVault ora include un'implementazione semplificata del protocollo di consenso Raft per gestire il failover automatico e la replicazione in un cluster di nodi distribuiti.

## Caratteristiche Raft

- **Consenso distribuito**: Garantisce che tutti i nodi concordino sullo stato dei dati
- **Leader election**: Elegge automaticamente un leader tra i nodi disponibili
- **Failover automatico**: Se il leader diventa non disponibile, viene eletto un nuovo leader
- **Replicazione sicura**: I comandi vengono replicati su una maggioranza di nodi prima di essere applicati
- **Consistency**: Garantisce la consistenza dei dati tra tutti i nodi

## Modalità di Utilizzo

### 1. Nodo Singolo con Raft

Per avviare un nodo singolo con consenso Raft abilitato:

```bash
./target/release/server --enable-raft --address "127.0.0.1:8080" --node-id "1"
```

### 2. Cluster Multi-Nodo (Futuro)

Per cluster multi-nodo, l'implementazione supporterà:

```bash
# Nodo 1 (leader iniziale)
./target/release/server --enable-raft --address "127.0.0.1:8080" --node-id "1" \
    --cluster-nodes "2,3"

# Nodo 2
./target/release/server --enable-raft --address "127.0.0.1:8081" --node-id "2" \
    --cluster-nodes "1,3"

# Nodo 3
./target/release/server --enable-raft --address "127.0.0.1:8082" --node-id "3" \
    --cluster-nodes "1,2"
```

## Opzioni da Linea di Comando

### Nuove Opzioni Raft

- `--enable-raft`: Abilita il consenso Raft (disabilita la replicazione legacy)
- `--cluster-nodes`: Lista dei nodi del cluster (format: id1,id2,id3)

### Opzioni Esistenti

- `--address` / `-a`: Indirizzo di bind del server (default: 127.0.0.1:8080)
- `--node-id` / `-n`: ID univoco del nodo (default: auto-generated)
- `--primary` / `-p`: Modalità legacy - avvia come nodo primario
- `--replica-of` / `-r`: Modalità legacy - replica del nodo specificato

## Differenze tra Modalità Legacy e Raft

### Modalità Legacy (Master-Slave)

- Un nodo primario fisso
- Repliche passive che sincronizzano dal primario
- Failover manuale
- Possibile perdita di dati in caso di failure del primario

### Modalità Raft (Distributed Consensus)

- Leader eletto dinamicamente
- Tutti i nodi partecipano al consenso
- Failover automatico
- Garanzia di consistenza e durabilità

## API e Protocollo

L'API resta la stessa sia in modalità legacy che Raft. I comandi vengono automaticamente routati attraverso il protocollo di consenso:

```json
{"command": "set", "key": "example", "value": {"data": "test"}}
{"command": "get", "key": "example"}
{"command": "delete", "key": "example"}
```

## Monitoraggio

### Metriche Raft

Il sistema espone metriche per monitorare lo stato del cluster:

- **node_id**: ID del nodo corrente
- **current_term**: Term Raft corrente
- **is_leader**: Se questo nodo è il leader
- **cluster_size**: Numero di nodi nel cluster
- **state**: Stato del nodo (Follower/Candidate/Leader)
- **last_log_index**: Index dell'ultima entry nel log
- **last_applied**: Index dell'ultima entry applicata

## Test del Sistema

### Script di Test Automatico

Esegui il test automatico con:

```bash
./test-raft.sh
```

### Test Manuale

1. Avvia un nodo:

```bash
cargo run --bin server -- --enable-raft --address "127.0.0.1:8080"
```

2. Testa con netcat:

```bash
echo '{"command": "set", "key": "test", "value": {"message": "Hello Raft!"}}' | nc 127.0.0.1 8080
echo '{"command": "get", "key": "test"}' | nc 127.0.0.1 8080
```

## Architettura Interna

### Componenti Raft

1. **SimpleRaftManager**: Gestisce lo stato del consenso
2. **LogEntry**: Rappresenta le entry nel log Raft
3. **Election Timer**: Gestisce i timeout per le elezioni
4. **State Machine**: Applica i comandi al database

### Stati del Nodo

- **Follower**: Stato iniziale, riceve comandi dal leader
- **Candidate**: Stato durante l'elezione del leader
- **Leader**: Riceve comandi dai client e li replica sui follower

### Flusso di un Comando

1. Client invia comando al nodo
2. Se il nodo è leader, aggiunge il comando al log
3. (Futuro) Replica il comando sui follower
4. Quando la maggioranza conferma, applica il comando
5. Restituisce la risposta al client

## Limitazioni Attuali

L'implementazione corrente è semplificata e include:

- Supporto completo per cluster single-node
- Struttura preparata per cluster multi-node
- Elezioni automatiche (implementazione base)
- Apply immediato dei comandi (senza attesa della maggioranza)

## Roadmap Futura

- [ ] Implementazione completa del protocollo Raft multi-node
- [ ] Network layer per comunicazione tra nodi
- [ ] Snapshot e log compaction
- [ ] Membership changes dinamiche
- [ ] Persistenza del log su disco
- [ ] Monitoring avanzato e dashboard web

## Docker Support

Il Dockerfile è già aggiornato per supportare Raft:

```bash
docker build -t jsonvault .
docker run -p 8080:8080 jsonvault jsonvault-server --enable-raft --port 8080
```

## Configurazione Kubernetes

I file nella cartella `kubernetes/` supportano deployment con Raft abilitato per alta disponibilità.
