/-
  Senbonzakura Core — Typing rules

  Judgment: σ; Γ ⊢ e : T where σ is the instance environment
  and Γ is the variable context.

  Overloadable operators are typed via instance lookup.
  and/or remain hardcoded as Bool → Bool → Bool.
-/

import SenbonzakuraCore.Syntax

namespace SenbonzakuraCore

abbrev Context := List Ty

inductive HasType : InstanceEnv → Context → Expr → Ty → Prop where
  | intLit  : HasType σ Γ (Expr.intLit n) (Ty.base .int)
  | strLit  : HasType σ Γ (Expr.strLit s) (Ty.base .str)
  | boolLit : HasType σ Γ (Expr.boolLit b) (Ty.base .bool)
  | noneLit : HasType σ Γ Expr.noneLit (Ty.base .none)

  | var : Γ[n]? = some T →
      HasType σ Γ (Expr.var n) T

  | lam : HasType σ (T₁ :: Γ) body T₂ →
      HasType σ Γ (Expr.lam T₁ body) (Ty.arrow T₁ T₂)

  | app : HasType σ Γ e₁ (Ty.arrow T₁ T₂) →
      HasType σ Γ e₂ T₁ →
      HasType σ Γ (Expr.app e₁ e₂) T₂

  | letE : HasType σ Γ e₁ T₁ →
      HasType σ (T₁ :: Γ) e₂ T₂ →
      HasType σ Γ (Expr.letE T₁ e₁ e₂) T₂

  | binopInstance :
      op.isOverloadable = true →
      InstanceEnv.hasInstance σ op T₁ T₂ T₃ →
      HasType σ Γ e₁ T₁ →
      HasType σ Γ e₂ T₂ →
      HasType σ Γ (Expr.binop op e₁ e₂) T₃

  | binopLogic :
      op.isOverloadable = false →
      HasType σ Γ e₁ (Ty.base .bool) →
      HasType σ Γ e₂ (Ty.base .bool) →
      HasType σ Γ (Expr.binop op e₁ e₂) (Ty.base .bool)

  | tuple : (hlen : es.length = Ts.length) →
      (∀ i (h : i < es.length), HasType σ Γ (es.get ⟨i, h⟩) (Ts.get ⟨i, by omega⟩)) →
      HasType σ Γ (Expr.tuple es) (Ty.prod Ts)

  | proj : HasType σ Γ e (Ty.prod Ts) →
      (h : i < Ts.length) →
      HasType σ Γ (Expr.proj e i) (Ts.get ⟨i, h⟩)

def InstanceEnv.wellFormed (env : InstanceEnv) : Prop :=
  ∀ e ∈ env, e.op.isOverloadable = true

end SenbonzakuraCore
