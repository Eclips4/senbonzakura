/-
  Senbonzakura Core — Syntax

  Core calculus with typeclass-based operator overloading.
  Typeclasses are modeled via an instance environment that maps
  (operator, input types) to output types, mirroring the transpiler's
  instance resolution.
-/

namespace SenbonzakuraCore

inductive BaseTy where
  | int | str | bool | none
  deriving DecidableEq, Repr

inductive Ty where
  | base : BaseTy → Ty
  | arrow : Ty → Ty → Ty
  | prod : List Ty → Ty

inductive BinOp where
  | add | sub | mul | div | mod
  | eq | lt
  | and | or
  deriving DecidableEq, Repr

/-- and/or are control flow, not overloadable via instances. -/
def BinOp.isOverloadable : BinOp → Bool
  | .add | .sub | .mul | .div | .mod | .eq | .lt => true
  | .and | .or => false

structure InstanceEntry where
  op : BinOp
  leftTy : Ty
  rightTy : Ty
  resultTy : Ty

abbrev InstanceEnv := List InstanceEntry

/-- Membership-based instance lookup (avoids needing DecidableEq on Ty). -/
def InstanceEnv.hasInstance (env : InstanceEnv) (op : BinOp) (l r out : Ty) : Prop :=
  { op := op, leftTy := l, rightTy := r, resultTy := out } ∈ env

inductive Expr where
  | intLit : Int → Expr
  | strLit : String → Expr
  | boolLit : Bool → Expr
  | noneLit : Expr
  | var : Nat → Expr
  | lam : Ty → Expr → Expr
  | app : Expr → Expr → Expr
  | letE : Ty → Expr → Expr → Expr
  | binop : BinOp → Expr → Expr → Expr
  | tuple : List Expr → Expr
  | proj : Expr → Nat → Expr

end SenbonzakuraCore
