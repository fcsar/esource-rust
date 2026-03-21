# RFC: edriven — Financial Ledger com CQRS + Event Sourcing

## Contexto e Motivação

Projeto de estudo para aprender Rust, CQRS, Event Sourcing e PostgreSQL avançado através da construção de um Ledger Financeiro — sistema onde cada movimentação (crédito/débito) é um evento imutável, e a leitura (saldo, extrato) vem de read models separadas, atualizadas de forma assíncrona.

## Arquitetura Geral

O sistema segue dois padrões complementares:

- **Event Sourcing**: estado é derivado de uma sequência de eventos imutáveis, nunca armazenado diretamente via UPDATE
- **CQRS**: escrita (commands) e leitura (queries) são separadas em caminhos independentes

### Fluxo de Escrita (Command Side)

```
HTTP Request (POST /accounts, POST /deposit, POST /withdraw)
    │
    ▼
Command (enum: CreateAccount | DepositMoney | WithdrawMoney)
    │
    ▼
Command Handler
    ├── Valida regras de negócio (saldo suficiente, conta existe)
    ├── Gera Event (enum: AccountCreated | MoneyDeposited | MoneyWithdrawn)
    │
    ▼
BEGIN transaction
    INSERT INTO ledger_events (event imutável)
    INSERT INTO event_outbox (status: pending)
COMMIT
```

### Fluxo de Projeção (Async)

```
Projection Worker (loop assíncrono)
    │
    ├── SELECT FROM event_outbox WHERE status = 'pending'
    ├── match no tipo do evento
    ├── Atualiza read model correspondente
    └── UPDATE event_outbox SET status = 'done'
```

### Fluxo de Leitura (Query Side)

```
HTTP Request (GET /balance/:id, GET /statement/:id)
    │
    ▼
Query Handler
    │
    ▼
SELECT FROM account_balances / transaction_history
    │
    ▼
JSON Response
```

### Diagrama Completo

```
                    ┌─────────────────────────────────┐
                    │         Command Side             │
  POST /deposit ──▶ │  Validate ──▶ Event Store (PG)   │
  POST /withdraw ─▶ │              + Outbox (PG)       │
  POST /accounts ─▶ │                                  │
                    └──────────────┬──────────────────┘
                                   │
                                   │ eventos pendentes
                                   ▼
                    ┌─────────────────────────────────┐
                    │       Projection Worker          │
                    │  Poll outbox ──▶ match event     │
                    │  ──▶ update read models          │
                    └──────────────┬──────────────────┘
                                   │
                                   ▼
                    ┌─────────────────────────────────┐
                    │          Query Side              │
  GET /balance ───▶ │  account_balances               │
  GET /statement ─▶ │  transaction_history            │
                    └─────────────────────────────────┘
```

## Padrões de Projeto

| Padrão | Onde é usado | Por quê |
|---|---|---|
| **Newtype** | `AccountId(Uuid)`, `Amount(i64)` | Compilador impede troca acidental de IDs |
| **Enum com dados** | `Command`, `Event` | Match exhaustive garante que todo caso é tratado |
| **Result<T, E>** | Command handlers, repositórios | Erros são valores, impossível ignorar silenciosamente |
| **Trait como contrato** | `EventStore`, `ReadModelRepository` | Permite trocar implementação (PG vs in-memory pra testes) |
| **Builder** | Configuração do app | Construção legível de structs complexas |
| **Repository** | Separado em EventStore + ReadModelRepository | Reflete a separação CQRS no código |
| **Outbox** | Tabela `event_outbox` | Garante entrega de eventos mesmo com falha de processo |

## Schema do Banco de Dados

### ledger_events (Event Store — append-only, imutável)

| Coluna | Tipo | Descrição |
|---|---|---|
| id | UUID PK | Identificador único do evento |
| account_id | UUID NOT NULL | Conta associada |
| event_type | VARCHAR NOT NULL | Tipo do evento (AccountCreated, MoneyDeposited, MoneyWithdrawn) |
| payload | JSONB NOT NULL | Dados do evento |
| created_at | TIMESTAMPTZ NOT NULL DEFAULT now() | Quando o evento ocorreu |

Constraints: sem UPDATE, sem DELETE. Apenas INSERT. Índice em `(account_id, created_at)`.

### event_outbox (Fila de entrega)

| Coluna | Tipo | Descrição |
|---|---|---|
| id | UUID PK | Identificador da entrada |
| event_id | UUID NOT NULL REFERENCES ledger_events(id) | Evento associado |
| status | VARCHAR NOT NULL DEFAULT 'pending' | pending → processing → done |
| created_at | TIMESTAMPTZ NOT NULL DEFAULT now() | Quando entrou na fila |
| processed_at | TIMESTAMPTZ | Quando foi processado |

Índice em `(status, created_at)` para o worker.

### account_balances (Read Model)

| Coluna | Tipo | Descrição |
|---|---|---|
| account_id | UUID PK | Conta |
| owner | VARCHAR NOT NULL | Dono da conta |
| balance | BIGINT NOT NULL DEFAULT 0 | Saldo em centavos |
| last_event_id | UUID NOT NULL | Último evento processado |
| updated_at | TIMESTAMPTZ NOT NULL | Última atualização |

### transaction_history (Read Model)

| Coluna | Tipo | Descrição |
|---|---|---|
| id | UUID PK | Identificador da transação |
| account_id | UUID NOT NULL | Conta associada |
| event_type | VARCHAR NOT NULL | Tipo da movimentação |
| amount | BIGINT NOT NULL | Valor em centavos |
| balance_after | BIGINT NOT NULL | Saldo após a movimentação |
| created_at | TIMESTAMPTZ NOT NULL | Quando ocorreu |

Índice em `(account_id, created_at DESC)` para extrato paginado.

## API Endpoints

### Commands (escrita)

| Método | Rota | Body | Retorno |
|---|---|---|---|
| POST | /accounts | `{ "owner": "string" }` | `{ "account_id": "uuid", "event": "AccountCreated" }` |
| POST | /accounts/:id/deposit | `{ "amount": integer }` | `{ "event_id": "uuid", "event": "MoneyDeposited" }` |
| POST | /accounts/:id/withdraw | `{ "amount": integer }` | `{ "event_id": "uuid", "event": "MoneyWithdrawn" }` |

### Queries (leitura)

| Método | Rota | Retorno |
|---|---|---|
| GET | /accounts/:id/balance | `{ "account_id": "uuid", "balance": integer, "updated_at": "datetime" }` |
| GET | /accounts/:id/statement | `[{ "event_type": "string", "amount": integer, "balance_after": integer, "created_at": "datetime" }]` |

### Operacional

| Método | Rota | Retorno |
|---|---|---|
| POST | /accounts/:id/snapshot | Reconstrói saldo via replay de todos os eventos |
| GET | /health | Status do serviço e conexão com banco |

## Stack Técnica

| Crate | Versão | Papel |
|---|---|---|
| axum | 0.8 | Framework HTTP (handlers, routing, extractors) |
| tokio | 1 | Runtime async (event loop, spawn, timers) |
| tower-http | 0.6 | Middleware HTTP (CORS, tracing de requests) |
| sqlx | 0.8 | Queries async com verificação em tempo de compilação |
| serde + serde_json | 1 | Serialização/deserialização JSON |
| uuid | 1 | Geração de IDs únicos (v4) |
| thiserror | 2 | Erros de domínio tipados |
| anyhow | 1 | Erros genéricos de infraestrutura |
| chrono | 0.4 | Manipulação de datas/timestamps |
| tracing + tracing-subscriber | 0.1/0.3 | Logging estruturado com contexto |
| dotenvy | 0.15 | Variáveis de ambiente (.env) |

### Futuro (Fase 4)

| Crate | Papel |
|---|---|
| rdkafka | Cliente Kafka — substitui Outbox por pub/sub |

## Fases de Implementação

### Fase 1 — Fundação
- Setup do projeto (Cargo.toml, linter, formatter) ✅
- Modelagem de domínio em Rust (structs, enums, newtypes)
- Conexão com PostgreSQL via sqlx
- Migrations (criar tabelas)

### Fase 2 — Command Side
- Command handlers (CreateAccount, DepositMoney, WithdrawMoney)
- Event Store (append em ledger_events + outbox)
- Endpoints HTTP de escrita
- Validação e error handling

### Fase 3 — Projection Worker
- Worker assíncrono com polling na event_outbox
- Projeção de eventos para account_balances
- Projeção de eventos para transaction_history
- Marcação de eventos como processados

### Fase 4 — Query Side + Kafka
- Endpoints de leitura (balance, statement)
- Endpoint de snapshot/replay
- Health check
- Migração do Outbox para Kafka

### Fase 5 — Observabilidade
- Tracing distribuído por request
- Métricas do worker (lag, throughput)
- Endpoint de métricas
