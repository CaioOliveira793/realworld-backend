## Custom headers

Custom headers are used to modify the request behavior

### Accept-Modified-After

Methods: `POST`, `PUT`

O uso deste header faz com que a requisição retorne uma resposta de sucesso caso a data de modificação do recurso conforme a intenção da requisição (criação ou atualização) seja posterior a data especificada e a requisição seja equivalente a usada para produzir o recurso.

O comportamento do `Accept-Modified-After` é complementar ao headers que realizam requisições condicionais como `If-None-Match`, `If-Match` e `If-Unmodified-Since`, sendo verificado após a falha da precondição.

O `Accept-Modified-After` possui um range de datas válidas em GMT de `$time::now - $ACCEPT_MODIFIED_AFTER_LIMIT` até `$time::now`, onde `$ACCEPT_MODIFIED_AFTER_LIMIT = 24h`. Esta limitação é usada para definir a expiração da validação de retentativas.

## Methods

### PUT

`PUT` method can be used to create and update resources, although its behavior is changed based on the **single required conditional header** present in the request.

#### If-None-Match

When used with `If-None-Match`, the request is semantically creating a resource, since the request precondition is the resource ID does not match. Based on the resource ID presence, the request will short return with `Precondition Failed` or continue to execute.

#### If-Match

`If-Match` implies an update semantic for `PUT`, verifying if the current resource matches the entity tag passed in the header.

#### If-Unmodified-Since

This precondition is used to update the resource if it is not modified after the date specified.

### POST

`POST` method is only used to create resources, which do **not require any conditional request header**, and are ignored when present.

This method is suitable for creating only entities and diverging body schemas for creating and updating requests.

## Error

```rust
struct PreconditionFailed {
	resource_id: Uuid,
	created: DateTime,
	updated: Option<DateTime>,
	version: u32,
}

struct Forbbiden {}

struct Conflict {
	resource_id: Uuid,
	created: DateTime,
	updated: Option<DateTime>,
	version: u32,
}
```

## Scenarios

1. criar nova entidade, id já existe
2. criar nova entidade, id já existe e resposta não é entregue, retentativa realizada
3. criar nova entidade, entidade criada e resposta não é entregue, retentativa realizada
4. criar nova entidade, entidade criada e resposta não é entregue, entidade atualizada por outro usuário, retentativa realizada
5. criar nova entidade, entidade criada e resposta não é entregue, entidade deletada por outro usuário, retentativa realizada

### 1. Scenario

Através do header `If-None-Match` é verificado que a entidade existe.

Caso a data de criação seja após a data de `Accept-Modified-After`, e sendo verificado que a requisição não pode reproduzir o mesmo recurso presente, é retornado `PreconditionFailed`, indicando conflito de ID.

Caso a data de criação seja após a data de `Accept-Modified-After`, e sendo verificado que a requisição pode reproduzir o mesmo recurso presente, é retornado `OK` com a resposta de sucesso.

> Este é um caso de conflito de id dentro do tempo limite de `ACCEPT_CREATED_AFTER_LIMIT` que produziu a mesma requisição. A chance de ocorrer acidentalmente é quase nula.

Caso a data de criação seja anterior a data de `Accept-Modified-After`, é retornando erro `PreconditionFailed`, indicando conflito de ID.

### 2. Scenario

A retentativa retonará a mesma resposta da primeira requisição no primeiro cenario (`PreconditionFailed`), indicando conflito de ID.

### 3. Scenario

Ao processar novamente a requisição é verificado a existencia de uma entidade com o mesmo ID.

Caso tenha sido criada após `Accept-Modified-After`, é verificado que a requisição pode reproduzir o mesmo recurso presente, retornando `OK` com a resposta de sucesso.

Caso tenha sido criada préviamente a `Accept-Modified-After`, é retornando erro `PreconditionFailed`, indicando conflito de ID.

> Este caso pode ocorrer devido ao `ACCEPT_CREATED_AFTER_LIMIT`, que marca a expiração do tempo de retentativas.

### 4. Scenario

Ao processar novamente a requisição é verificado a existencia de uma entidade com o mesmo ID.

Caso tenha sido criada após `Accept-Modified-After`, e sendo verificado que a requisição pode reproduzir o mesmo recurso presente, é retornado `OK` com a resposta de sucesso.

Caso tenha sido criada após `Accept-Modified-After`, e sendo verificado que a requisição não pode reproduzir o mesmo recurso presente, é retornando erro `PreconditionFailed`, indicando conflito de ID.

> Mesmo atualizando a entidade, pode ser feito um hash da request que criou a entidade para comparar em casos de retentativa.

Caso tenha sido criada préviamente a `Accept-Modified-After`, é retornando erro `PreconditionFailed`, indicando conflito de ID.

> Este caso pode ocorrer devido ao `ACCEPT_CREATED_AFTER_LIMIT`, que marca a expiração do tempo de retentativas.

### 5. Scenario

Ao realizar a retentativa o recurso será criado novamente.

## Notes

```
services SHOULD:
1) support PUT /collection/{guid}, not POST /collection/ to create resources
1) require If-None-Match: * header on create (PUT) to prevent key collisions and return 412 Precondition Failed if the PUT fails
2) return ETag: {etag-hash} header on GET to ensure content idempotence
3) require If-Match: {etag-hash} header on update (PUT) to prevent content collisions and return 412 Precondition Failed if the PUT fails

clients SHOULD:
1) generate their own resource keys (you mention this)
2) send If-None-Match: * on create (PUT) & accept 412 Precondition Failed in response
2) send If-None-Match: {guid-hash} on GET & accept 204 No Content in response
3) send If-Match {guid-hash} headers on update (PUT) & accept 412 Precondition Failed in response
```

```
let first_try = $time::now;

PUT /iam/user/123 HTTP/1.1
If-None-Match: *
Accept-Modified-After: first_try

{
	"email":"user@email.com",
	"password":"12345678"
}
```
