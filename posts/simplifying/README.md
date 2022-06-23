---
id: 7d621f0a-1032-46c4-8f9a-bc38acf51673
title: Simplifying things
date: "2022-06-22"
tags: [programming, thinking]
slug: simplifying-things
---

Simplifying things is one of the most underrated ideas that people have when is
doing anything. But understand why you need it and how you can do it, can improve
a lot of things in your daily workflow.

An **abstraction** is a nice way to simplify things. But what is an abstraction?
It's the process of removing implementation details of the final result. Abstracting
things is a process that we, humans, do every day. You don't need to expose every
detail from something, abstract it in a way that turns more easily to understand is
a good way to do it.

We can talk that **automation** is a way to abstract things too, in general I agreed with
that. But, automate things is a way to turn it more simplified. For example, you
can automate your daily workflow in some ways:

- Remove spams from your email inbox;
- Schedule things in your agenda;
- Routine;
- Write things;

And lot of other things that you can do to help your daily workdflow. I think that we
can agree that it's a good way to simplifcate your day, right?

## Simplification as code

But thinking in programming, how we can simplify our model? What things we should do to
improve our daily coding sessions? How we can turn our work easier? There are some thigns
that we can do to improve it.

One of the things that we could understand is: we don't need every feature from some package,
framework, etc. It's a good idea know about the existence of this function, but think if it's
really necessary the usage of it on the scope of your project is a way to do simplification even
before write a line of code.

An example about it is: did you know that Mongoose (an ODM to handle MongoDb) has a way to manage
versions of your documents? If you didn't know, yes. There's a way to do it. The `__v` is a simple
property that stores an incremental integer and the propose behind it is: every time that you change the
structure of this document, `__v` will be incremented.

In some cases, we can treat it to do something based on the version of this document. Write a migration,
treat some internal logic or other things like it. But, thinking in your code, do you think that it's a
good idea to handle something like it? Thinking in every thing that you need to treat when write a code
to handle it, it's a good idea to do it? With every conditional branch, every corner case, every minimal
detail that you need to trait when is handle some sensitive data like a document in your database.

When you use a feature of the library, framework or something like that, and doesn't think in the consequences
of it, it's the opposite of simplification, you are turn your work harder from now on. You just complicated the
work that you or other person will have to do when maintain this code, and over time this work will be more and
more complicated because the complexity will grow, and thinking in some months or a year, you or the person that
will fix the bug that appears in your code will say that this code has been simplified or not?
