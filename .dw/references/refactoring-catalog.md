# Catálogo de Refatoração — Exemplos Antes/Depois

Referência para `/dw-refactor` e `/dw-brainstorm --mode=refactor-audit`. Baseado no catálogo de Fowler.

## 1. Função Longa → Extract Function

**Smell:** Função com >15 linhas de lógica, múltiplas responsabilidades.

```typescript
// ❌ ANTES: 30+ linhas fazendo validação, transformação e persistência
async function processOrder(order: Order) {
  if (!order.items.length) throw new Error("Empty order");
  if (order.total < 0) throw new Error("Invalid total");
  if (!order.customer) throw new Error("No customer");
  
  const discount = order.customer.isPremium 
    ? order.total * 0.1 
    : order.total > 100 ? order.total * 0.05 : 0;
  const tax = (order.total - discount) * 0.15;
  const finalTotal = order.total - discount + tax;
  
  order.discount = discount;
  order.tax = tax;
  order.total = finalTotal;
  order.status = "processed";
  
  await db.orders.update(order.id, order);
  await emailService.send(order.customer.email, "Order processed", { order });
}

// ✅ DEPOIS: Cada função com uma responsabilidade
async function processOrder(order: Order) {
  validateOrder(order);
  const pricing = calculatePricing(order);
  const processed = applyPricing(order, pricing);
  await persistOrder(processed);
  await notifyCustomer(processed);
}
```

## 2. Feature Envy → Move Method

**Smell:** Função acessa dados de outro objeto mais que os próprios.

```typescript
// ❌ ANTES: calculateShipping conhece demais sobre Address
function calculateShipping(order: Order) {
  const addr = order.address;
  if (addr.country === "BR" && addr.state === "SP") return 5.99;
  if (addr.country === "BR") return 9.99;
  return 19.99 + (addr.isRemote ? 10 : 0);
}

// ✅ DEPOIS: Address é dono da lógica de frete
class Address {
  getShippingCost(): number {
    if (this.country === "BR" && this.state === "SP") return 5.99;
    if (this.country === "BR") return 9.99;
    return 19.99 + (this.isRemote ? 10 : 0);
  }
}
```

## 3. Obsessão por Primitivos → Value Object

**Smell:** Usar strings/números crus para conceitos do domínio (emails, dinheiro, datas).

```typescript
// ❌ ANTES: email é apenas uma string em todo lugar
function sendEmail(to: string, subject: string) {
  if (!to.includes("@")) throw new Error("Invalid email");
}

// ✅ DEPOIS: Email é um value object com validação embutida
class Email {
  constructor(private readonly value: string) {
    if (!value.includes("@")) throw new Error("Invalid email");
  }
  toString() { return this.value; }
}

function sendEmail(to: Email, subject: string) { /* ... */ }
```

## 4. Lógica Duplicada → Extrair Utilitário Compartilhado

**Smell:** Mesmas 3+ linhas de lógica aparecem em múltiplos locais.

```typescript
// ❌ ANTES: formatação de data repetida em 4 componentes
const formatted = `${date.getFullYear()}-${String(date.getMonth()+1).padStart(2,"0")}-${String(date.getDate()).padStart(2,"0")}`;

// ✅ DEPOIS: utilitário único
function formatDate(date: Date): string {
  return date.toISOString().split("T")[0];
}
```

## 5. God Component → Dividir por Responsabilidade

**Smell:** Componente React com 200+ linhas, múltiplos useEffects, responsabilidades misturadas.

```tsx
// ❌ ANTES: UserDashboard faz fetch, filtragem, renderização e modais
function UserDashboard() {
  // 50 linhas de state + effects
  // 30 linhas de handlers
  // 120 linhas de JSX com condições inline
}

// ✅ DEPOIS: container + presentation + hook
function useUserDashboard() { /* data fetching + state */ }
function UserFilters({ filters, onChange }) { /* UI de filtros */ }
function UserTable({ users, onSelect }) { /* UI de tabela */ }
function UserDashboard() {
  const { users, filters, setFilters } = useUserDashboard();
  return (
    <>
      <UserFilters filters={filters} onChange={setFilters} />
      <UserTable users={users} />
    </>
  );
}
```

## 6. Condicional Complexa → Strategy Pattern / Early Return

**Smell:** Cadeia de if/else aninhados com 4+ branches.

```typescript
// ❌ ANTES: condicionais aninhadas
function getPrice(user: User, product: Product) {
  if (user.type === "premium") {
    if (product.category === "electronics") {
      return product.price * 0.8;
    } else {
      return product.price * 0.9;
    }
  } else if (user.type === "wholesale") {
    return product.price * 0.7;
  } else {
    return product.price;
  }
}

// ✅ DEPOIS: mapa de estratégias + early return
const DISCOUNT_STRATEGIES: Record<string, (p: Product) => number> = {
  premium: (p) => p.category === "electronics" ? 0.8 : 0.9,
  wholesale: () => 0.7,
  standard: () => 1.0,
};

function getPrice(user: User, product: Product) {
  const discount = DISCOUNT_STRATEGIES[user.type] ?? DISCOUNT_STRATEGIES.standard;
  return product.price * discount(product);
}
```

## Guia de Priorização

| Severidade | Critério | Ação |
|------------|----------|------|
| **P0 - Crítico** | Risco de segurança, corrupção de dados, contrato de API quebrado | Corrigir imediatamente |
| **P1 - Alto** | >3 duplicações, god objects, código não-testável | Corrigir no sprint atual |
| **P2 - Médio** | Funções longas, obsessão por primitivos, feature envy | Agendar para refatoração |
| **P3 - Baixo** | Problemas menores de naming, pequenas duplicações, estilo | Corrigir oportunisticamente |
