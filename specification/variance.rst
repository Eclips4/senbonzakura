========
Variance
========

The ``senbonzakura`` language really likes the subtyping rules.
The syntax in language as follows::

   data Animal[-T]:
      attr: T

There are three kinds of variance:
- ``-T`` means contravariance, which allows using the ``T`` itself and it's supertypes.
- ``T`` means invariance, no subtypes or supertypes are allowed.
- ``+T`` means covariance wich allows using the ``T`` itself and it's subtypes.

What is subtyping?
Subtyping is a relationship between types where one type can be safely used in place
of another type.
If ``Cat`` is a subtype of ``Animal``, then anywhere where you expect ``Animal``, you
can safely pass a ``Cat``.

The term subtyping has a two directions, one "to-the-down" and the second one is
"to-the-top".
When we're talking about "to-the-top", it oftens means supertype.


Let's imagine a simple hierarchy::

	Creature
           └──Animal
                  └──Cat
                      └──Kitty

Read that as: ``Kitty <: Cat <: Animal <: Creature``, where's the ``<:`` means "is a subtype of".


..code-block:: text

   data Box[+T]:  # A covarian over T
      value: T

So, if ``Cat <: Animal``, then ``Box[Cat] <: Box[Animal] <: Box[Creature]``.
