/-
  Senbonzakura Core — Type Soundness

  Progress + Preservation = well-typed programs don't go wrong.

  Three axioms form the trust boundary with the runtime:
  - instance_progress: declared instances can evaluate
  - instance_eval_preservation: evaluation produces well-typed results
  - substitution_lemma: standard (TAPL 9.3.8)
-/

import SenbonzakuraCore.Syntax
import SenbonzakuraCore.Semantics
import SenbonzakuraCore.Typing

namespace SenbonzakuraCore

-- Axioms

axiom instance_progress {env : InstanceEnv} {op : BinOp} {T₁ T₂ T₃ : Ty}
    {v₁ v₂ : Expr} :
    InstanceEnv.hasInstance env op T₁ T₂ T₃ →
    IsValue v₁ → IsValue v₂ →
    HasType env [] v₁ T₁ → HasType env [] v₂ T₂ →
    ∃ r, Step (Expr.binop op v₁ v₂) r

axiom instance_eval_preservation {env : InstanceEnv} {Γ : Context}
    {op : BinOp} {T₁ T₂ T₃ : Ty} {v₁ v₂ r : Expr} :
    InstanceEnv.hasInstance env op T₁ T₂ T₃ →
    HasType env Γ v₁ T₁ → HasType env Γ v₂ T₂ →
    IsValue v₁ → IsValue v₂ →
    Step (Expr.binop op v₁ v₂) r →
    HasType env Γ r T₃

axiom substitution_lemma {env : InstanceEnv} {Γ : Context} {e s : Expr} {S T : Ty} :
    HasType env (S :: Γ) e T → HasType env Γ s S → HasType env Γ (subst 0 s e) T

-- Instance weakening: adding instances never breaks existing code.

theorem instance_weakening {env env' : InstanceEnv} {Γ : Context} {e : Expr} {T : Ty}
    (hsub : ∀ entry, entry ∈ env → entry ∈ env')
    (ht : HasType env Γ e T) : HasType env' Γ e T := by
  induction ht with
  | intLit => exact HasType.intLit
  | strLit => exact HasType.strLit
  | boolLit => exact HasType.boolLit
  | noneLit => exact HasType.noneLit
  | var h => exact HasType.var h
  | lam _ ih => exact HasType.lam (ih hsub)
  | app _ _ ih₁ ih₂ => exact HasType.app (ih₁ hsub) (ih₂ hsub)
  | letE _ _ ih₁ ih₂ => exact HasType.letE (ih₁ hsub) (ih₂ hsub)
  | binopInstance hov hinst _ _ ih₁ ih₂ =>
      exact HasType.binopInstance hov (hsub _ hinst) (ih₁ hsub) (ih₂ hsub)
  | binopLogic hov _ _ ih₁ ih₂ =>
      exact HasType.binopLogic hov (ih₁ hsub) (ih₂ hsub)
  | tuple hlen _ ih =>
      exact HasType.tuple hlen (fun i h => ih i h hsub)
  | proj _ hlt ih =>
      exact HasType.proj (ih hsub) hlt

-- Canonical forms

theorem canonical_arrow {env : InstanceEnv} {e : Expr} {T₁ T₂ : Ty}
    (hv : IsValue e) (ht : HasType env [] e (Ty.arrow T₁ T₂)) :
    ∃ body, e = Expr.lam T₁ body := by
  cases hv with
  | lam => cases ht with | lam _ => exact ⟨_, rfl⟩
  | intLit => cases ht
  | strLit => cases ht
  | boolLit => cases ht
  | noneLit => cases ht
  | tuple _ => cases ht

theorem canonical_prod {env : InstanceEnv} {e : Expr} {Ts : List Ty}
    (hv : IsValue e) (ht : HasType env [] e (Ty.prod Ts)) :
    ∃ es, e = Expr.tuple es := by
  cases hv with
  | tuple _ => cases ht with | tuple _ _ => exact ⟨_, rfl⟩
  | intLit => cases ht
  | strLit => cases ht
  | boolLit => cases ht
  | noneLit => cases ht
  | lam => cases ht

theorem canonical_bool {env : InstanceEnv} {e : Expr}
    (hv : IsValue e) (ht : HasType env [] e (Ty.base .bool)) :
    ∃ b, e = Expr.boolLit b := by
  cases hv with
  | boolLit => exact ⟨_, rfl⟩
  | intLit => cases ht
  | strLit => cases ht
  | noneLit => cases ht
  | lam => cases ht
  | tuple _ => cases ht

-- Helpers

theorem value_does_not_step {e e' : Expr} (hv : IsValue e) (hs : Step e e') : False := by
  induction hs with
  | appLam _ => cases hv
  | appFun _ _ => cases hv
  | appArg _ _ _ => cases hv
  | letVal _ => cases hv
  | letStep _ _ => cases hv
  | binopInt _ => cases hv
  | binopBool _ => cases hv
  | binopLeft _ _ => cases hv
  | binopRight _ _ _ => cases hv
  | projTuple _ _ => cases hv
  | projStep _ _ => cases hv
  | tupleStep hk _ ih =>
    cases hv with
    | tuple hfields => exact ih (hfields _ hk)

theorem intLit_not_bool {env : InstanceEnv} {Γ : Context} {n : Int}
    (ht : HasType env Γ (Expr.intLit n) (Ty.base .bool)) : False := by
  cases ht

theorem boolLit_not_int {env : InstanceEnv} {Γ : Context} {b : Bool}
    (ht : HasType env Γ (Expr.boolLit b) (Ty.base .int)) : False := by
  cases ht

-- Preservation

theorem preservation {env : InstanceEnv} {Γ : Context} {e e' : Expr} {T : Ty}
    (ht : HasType env Γ e T) (hs : Step e e') : HasType env Γ e' T := by
  induction hs generalizing T Γ with
  | appLam _ =>
    cases ht with
    | app ht₁ ht₂ =>
      cases ht₁ with
      | lam hbody => exact substitution_lemma hbody ht₂
  | appFun _ ih =>
    cases ht with
    | app ht₁ ht₂ => exact HasType.app (ih ht₁) ht₂
  | appArg _ _ ih =>
    cases ht with
    | app ht₁ ht₂ => exact HasType.app ht₁ (ih ht₂)
  | letVal _ =>
    cases ht with
    | letE ht₁ ht₂ => exact substitution_lemma ht₂ ht₁
  | letStep _ ih =>
    cases ht with
    | letE ht₁ ht₂ => exact HasType.letE (ih ht₁) ht₂
  | @binopInt op a b r heval =>
    cases ht with
    | binopInstance hov hinst ht₁ ht₂ =>
        exact instance_eval_preservation hinst ht₁ ht₂ IsValue.intLit IsValue.intLit
          (Step.binopInt heval)
    | binopLogic hov ht₁ ht₂ =>
        exact absurd ht₁ (by intro h; cases h)
  | @binopBool op a b r heval =>
    cases ht with
    | binopInstance hov hinst ht₁ ht₂ =>
        exact instance_eval_preservation hinst ht₁ ht₂ IsValue.boolLit IsValue.boolLit
          (Step.binopBool heval)
    | binopLogic hov ht₁ ht₂ =>
        revert heval
        cases op <;> simp [BinOp.isOverloadable] at hov <;> simp [evalBoolLogic]
        all_goals (intro h; cases h; exact HasType.boolLit)
  | binopLeft _ ih =>
    cases ht with
    | binopInstance hov hinst ht₁ ht₂ =>
        exact HasType.binopInstance hov hinst (ih ht₁) ht₂
    | binopLogic hov ht₁ ht₂ =>
        exact HasType.binopLogic hov (ih ht₁) ht₂
  | binopRight _ _ ih =>
    cases ht with
    | binopInstance hov hinst ht₁ ht₂ =>
        exact HasType.binopInstance hov hinst ht₁ (ih ht₂)
    | binopLogic hov ht₁ ht₂ =>
        exact HasType.binopLogic hov ht₁ (ih ht₂)
  | tupleStep hk hstep ih =>
    cases ht with
    | tuple hlen hfields =>
      refine HasType.tuple (by simp [List.length_set]; exact hlen) (fun i hi => ?_)
      simp only [List.length_set] at hi
      simp only [List.get_eq_getElem, List.getElem_set]
      split
      · next heq =>
          simp only [List.get_eq_getElem, heq] at ih
          simp only [List.get_eq_getElem] at hfields
          exact ih (hfields i (by omega))
      · next hne =>
          simp only [List.get_eq_getElem] at hfields
          exact hfields i (by omega)
  | @projTuple es i hv hi =>
    cases ht with
    | @proj _ _ _ Ts _ hte hlt =>
      cases hte with
      | tuple hlen hfields =>
        exact hfields i (by omega)
  | projStep _ ih =>
    cases ht with
    | proj hte hlt => exact HasType.proj (ih hte) hlt

-- Progress

theorem binopLogic_progress {op : BinOp} {b₁ b₂ : Bool}
    (hov : op.isOverloadable = false) :
    ∃ e', Step (Expr.binop op (Expr.boolLit b₁) (Expr.boolLit b₂)) e' := by
  cases op <;> simp [BinOp.isOverloadable] at hov
  · exact ⟨_, Step.binopBool (show evalBoolLogic .and b₁ b₂ = some _ from rfl)⟩
  · exact ⟨_, Step.binopBool (show evalBoolLogic .or b₁ b₂ = some _ from rfl)⟩

theorem progress {env : InstanceEnv} {e : Expr} {T : Ty}
    (ht : HasType env [] e T) : IsValue e ∨ ∃ e', Step e e' := by
  generalize hΓ : ([] : Context) = Γ at ht
  induction ht with
  | intLit => left; exact IsValue.intLit
  | strLit => left; exact IsValue.strLit
  | boolLit => left; exact IsValue.boolLit
  | noneLit => left; exact IsValue.noneLit
  | var h => subst hΓ; simp at h
  | lam _ => left; exact IsValue.lam
  | app ht₁ ht₂ ih₁ ih₂ =>
    right
    have ih₁ := ih₁ hΓ; have ih₂ := ih₂ hΓ
    cases ih₁ with
    | inl hv₁ =>
      cases ih₂ with
      | inl hv₂ =>
        have ⟨body, hbody⟩ := canonical_arrow hv₁ (hΓ ▸ ht₁)
        subst hbody
        exact ⟨_, Step.appLam hv₂⟩
      | inr he₂ =>
        obtain ⟨e₂', he₂'⟩ := he₂
        exact ⟨_, Step.appArg hv₁ he₂'⟩
    | inr he₁ =>
      obtain ⟨e₁', he₁'⟩ := he₁
      exact ⟨_, Step.appFun he₁'⟩
  | letE _ _ ih₁ _ =>
    right
    have ih₁ := ih₁ hΓ
    cases ih₁ with
    | inl hv => exact ⟨_, Step.letVal hv⟩
    | inr he₁ =>
      obtain ⟨e₁', he₁'⟩ := he₁
      exact ⟨_, Step.letStep he₁'⟩
  | binopInstance hov hinst _ _ ih₁ ih₂ =>
    right
    have ih₁ := ih₁ hΓ; have ih₂ := ih₂ hΓ
    cases ih₁ with
    | inl hv₁ =>
      cases ih₂ with
      | inl hv₂ =>
        exact instance_progress hinst hv₁ hv₂ (hΓ ▸ ‹_›) (hΓ ▸ ‹_›)
      | inr he₂ =>
        obtain ⟨e₂', he₂'⟩ := he₂
        exact ⟨_, Step.binopRight hv₁ he₂'⟩
    | inr he₁ =>
      obtain ⟨e₁', he₁'⟩ := he₁
      exact ⟨_, Step.binopLeft he₁'⟩
  | binopLogic hov ht₁ ht₂ ih₁ ih₂ =>
    right
    have ih₁ := ih₁ hΓ; have ih₂ := ih₂ hΓ
    cases ih₁ with
    | inl hv₁ =>
      cases ih₂ with
      | inl hv₂ =>
        have ⟨b₁, hb₁⟩ := canonical_bool hv₁ (hΓ ▸ ht₁)
        have ⟨b₂, hb₂⟩ := canonical_bool hv₂ (hΓ ▸ ht₂)
        subst hb₁; subst hb₂
        exact binopLogic_progress hov
      | inr he₂ =>
        obtain ⟨e₂', he₂'⟩ := he₂
        exact ⟨_, Step.binopRight hv₁ he₂'⟩
    | inr he₁ =>
      obtain ⟨e₁', he₁'⟩ := he₁
      exact ⟨_, Step.binopLeft he₁'⟩
  | @tuple σ Γ' es Ts hlen hfields ih =>
    subst hΓ
    by_cases hall : ∀ i (h : i < es.length), IsValue (es.get ⟨i, h⟩)
    · left; exact IsValue.tuple hall
    · right
      have ⟨k, hk_rest⟩ := Classical.not_forall.mp hall
      have ⟨hk, hnv⟩ := Classical.not_forall.mp hk_rest
      have ihk := ih k hk rfl
      cases ihk with
      | inl hv => exact absurd hv hnv
      | inr he =>
        obtain ⟨e', he'⟩ := he
        exact ⟨_, Step.tupleStep hk he'⟩
  | proj hte hi ih =>
    right
    have ih := ih hΓ
    cases ih with
    | inl hv =>
      have ⟨es, hes⟩ := canonical_prod hv (hΓ ▸ hte)
      subst hes
      cases hte with
      | tuple hlen _ => exact ⟨_, Step.projTuple hv (by omega)⟩
    | inr he =>
      obtain ⟨e', he'⟩ := he
      exact ⟨_, Step.projStep he'⟩

-- Type safety

theorem type_safety {env : InstanceEnv} {e e' : Expr} {T : Ty}
    (ht : HasType env [] e T) (hstep : Step e e') :
    IsValue e' ∨ ∃ e'', Step e' e'' :=
  progress (preservation ht hstep)

-- Constrained generics: calling a constrained function with a valid instance is sound.

theorem constrained_instantiation_sound
    {env : InstanceEnv} {body : Expr} {T_concrete : Ty} {op : BinOp}
    (h_body : HasType
      ({ op := op, leftTy := T_concrete, rightTy := T_concrete,
         resultTy := T_concrete } :: env)
      [T_concrete] body T_concrete)
    (h_inst : InstanceEnv.hasInstance env op T_concrete T_concrete T_concrete) :
    HasType env [T_concrete] body T_concrete := by
  exact instance_weakening (fun entry hmem => by
    simp [List.mem_cons] at hmem
    cases hmem with
    | inl h => rw [h]; exact h_inst
    | inr h => exact h
  ) h_body

end SenbonzakuraCore
