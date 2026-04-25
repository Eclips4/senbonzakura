/-
  Senbonzakura Core — Small-step operational semantics

  Values, substitution, and the reduction relation.
  The typing rules guarantee instances exist, so reduction proceeds
  knowing the types are compatible.
-/

import SenbonzakuraCore.Syntax

namespace SenbonzakuraCore

inductive IsValue : Expr → Prop where
  | intLit  : IsValue (Expr.intLit n)
  | strLit  : IsValue (Expr.strLit s)
  | boolLit : IsValue (Expr.boolLit b)
  | noneLit : IsValue Expr.noneLit
  | lam     : IsValue (Expr.lam t body)
  | tuple   : (∀ i (h : i < es.length), IsValue (es.get ⟨i, h⟩)) →
               IsValue (Expr.tuple es)

def subst (j : Nat) (s : Expr) : Expr → Expr
  | Expr.var n => if n == j then s else Expr.var n
  | Expr.lam ty body => Expr.lam ty (subst (j + 1) s body)
  | Expr.app e₁ e₂ => Expr.app (subst j s e₁) (subst j s e₂)
  | Expr.letE ty e₁ e₂ => Expr.letE ty (subst j s e₁) (subst (j + 1) s e₂)
  | Expr.binop op e₁ e₂ => Expr.binop op (subst j s e₁) (subst j s e₂)
  | Expr.tuple es => Expr.tuple (es.map (subst j s))
  | Expr.proj e n => Expr.proj (subst j s e) n
  | e => e

def evalIntOp (op : BinOp) (a b : Int) : Option Expr :=
  match op with
  | .add => some (Expr.intLit (a + b))
  | .sub => some (Expr.intLit (a - b))
  | .mul => some (Expr.intLit (a * b))
  | .div => if b ≠ 0 then some (Expr.intLit (a / b)) else none
  | .mod => if b ≠ 0 then some (Expr.intLit (a % b)) else none
  | .eq  => some (Expr.boolLit (a == b))
  | .lt  => some (Expr.boolLit (decide (a < b)))
  | _ => none

def evalBoolLogic (op : BinOp) (a b : Bool) : Option Expr :=
  match op with
  | .and => some (Expr.boolLit (a && b))
  | .or  => some (Expr.boolLit (a || b))
  | _ => none

inductive Step : Expr → Expr → Prop where
  | appLam : IsValue v →
      Step (Expr.app (Expr.lam ty body) v) (subst 0 v body)
  | appFun : Step e₁ e₁' →
      Step (Expr.app e₁ e₂) (Expr.app e₁' e₂)
  | appArg : IsValue v₁ → Step e₂ e₂' →
      Step (Expr.app v₁ e₂) (Expr.app v₁ e₂')
  | letVal : IsValue v →
      Step (Expr.letE ty v e₂) (subst 0 v e₂)
  | letStep : Step e₁ e₁' →
      Step (Expr.letE ty e₁ e₂) (Expr.letE ty e₁' e₂)
  | binopInt : evalIntOp op a b = some r →
      Step (Expr.binop op (Expr.intLit a) (Expr.intLit b)) r
  | binopBool : evalBoolLogic op a b = some r →
      Step (Expr.binop op (Expr.boolLit a) (Expr.boolLit b)) r
  | binopLeft : Step e₁ e₁' →
      Step (Expr.binop op e₁ e₂) (Expr.binop op e₁' e₂)
  | binopRight : IsValue v₁ → Step e₂ e₂' →
      Step (Expr.binop op v₁ e₂) (Expr.binop op v₁ e₂')
  | tupleStep :
      (hk : k < es.length) →
      Step (es.get ⟨k, hk⟩) e' →
      Step (Expr.tuple es) (Expr.tuple (es.set k e'))
  | projTuple : IsValue (Expr.tuple es) →
      (h : i < es.length) →
      Step (Expr.proj (Expr.tuple es) i) (es.get ⟨i, h⟩)
  | projStep : Step e e' →
      Step (Expr.proj e n) (Expr.proj e' n)

end SenbonzakuraCore
