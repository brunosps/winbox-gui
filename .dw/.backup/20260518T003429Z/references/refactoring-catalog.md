# Refactoring Catalog — Before/After Examples

Reference for `/dw-refactoring-analysis`. Based on Fowler's refactoring catalog.

## 1. Long Function → Extract Function

**Smell:** Function with >15 lines of logic, multiple responsibilities.

```typescript
// ❌ BEFORE: 30+ lines doing validation, transformation, and persistence
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

// ✅ AFTER: Each function has one responsibility
async function processOrder(order: Order) {
  validateOrder(order);
  const pricing = calculatePricing(order);
  const processed = applyPricing(order, pricing);
  await persistOrder(processed);
  await notifyCustomer(processed);
}
```

## 2. Feature Envy → Move Method

**Smell:** Function accesses another object's data more than its own.

```typescript
// ❌ BEFORE: calculateShipping knows too much about Address
function calculateShipping(order: Order) {
  const addr = order.address;
  if (addr.country === "US" && addr.state === "CA") return 5.99;
  if (addr.country === "US") return 9.99;
  return 19.99 + (addr.isRemote ? 10 : 0);
}

// ✅ AFTER: Address owns its shipping logic
class Address {
  getShippingCost(): number {
    if (this.country === "US" && this.state === "CA") return 5.99;
    if (this.country === "US") return 9.99;
    return 19.99 + (this.isRemote ? 10 : 0);
  }
}
```

## 3. Primitive Obsession → Value Object

**Smell:** Using raw strings/numbers for domain concepts (emails, money, dates).

```typescript
// ❌ BEFORE: email is just a string everywhere
function sendEmail(to: string, subject: string) {
  if (!to.includes("@")) throw new Error("Invalid email");
  // ...
}

// ✅ AFTER: Email is a value object with built-in validation
class Email {
  constructor(private readonly value: string) {
    if (!value.includes("@")) throw new Error("Invalid email");
  }
  toString() { return this.value; }
}

function sendEmail(to: Email, subject: string) { /* ... */ }
```

## 4. Duplicated Logic → Extract Shared Utility

**Smell:** Same 3+ lines of logic appear in multiple places.

```typescript
// ❌ BEFORE: date formatting repeated in 4 components
const formatted = `${date.getFullYear()}-${String(date.getMonth()+1).padStart(2,"0")}-${String(date.getDate()).padStart(2,"0")}`;

// ✅ AFTER: single utility
function formatDate(date: Date): string {
  return date.toISOString().split("T")[0];
}
```

## 5. God Component → Split by Responsibility

**Smell:** React component with 200+ lines, multiple useEffects, mixed concerns.

```tsx
// ❌ BEFORE: UserDashboard handles data fetching, filtering, rendering, modals
function UserDashboard() {
  // 50 lines of state + effects
  // 30 lines of handlers
  // 120 lines of JSX with inline conditions
}

// ✅ AFTER: Split into container + presentation + hook
function useUserDashboard() { /* data fetching + state */ }
function UserFilters({ filters, onChange }) { /* filter UI */ }
function UserTable({ users, onSelect }) { /* table UI */ }
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

## 6. Complex Conditional → Strategy Pattern / Early Return

**Smell:** Nested if/else chains with 4+ branches.

```typescript
// ❌ BEFORE: nested conditionals
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

// ✅ AFTER: strategy map + early return
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

## Priority Assessment Guide

| Severity | Criteria | Action |
|----------|----------|--------|
| **P0 - Critical** | Security risk, data corruption, breaking API contract | Fix immediately |
| **P1 - High** | >3 duplications, god objects, untestable code | Fix in current sprint |
| **P2 - Medium** | Long functions, primitive obsession, feature envy | Schedule for refactoring |
| **P3 - Low** | Minor naming issues, small duplications, style | Fix opportunistically |
