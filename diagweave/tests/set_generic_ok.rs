use diagweave::{DiagnosticError, set};
use std::fmt::{Debug, Display};

set! {
    #[derive(Clone, Debug, PartialEq)]
    pub SetGenericA<T: Display + Debug + std::fmt::Debug + Send + Sync + 'static> = {
        #[display("item: {0}")]
        Item(T),
    }

    #[derive(Clone, Debug, PartialEq)]
    pub SetGenericB<T> where T: Display + Debug + std::fmt::Debug + Send + Sync + 'static = SetGenericA<T> | {
        #[display("custom failure")]
        Custom,
    }

    #[derive(Clone, Debug)]
    pub SetLifetime<'a: 'static> = {
        #[display("borrowed msg: {0}")]
        Borrowed(&'a str),
    }

    #[derive(Clone, Debug)]
    pub SetSuperLifetime<'a: 'static> = SetLifetime<'a> | {
        #[display("owned: {0}")]
        Owned(String),
    }
}

#[test]
fn test_generic_set_basic() {
    let a: SetGenericA<i32> = SetGenericA::Item(42);
    assert_eq!(a.to_string(), "item: 42");

    let b: SetGenericB<i32> = a.into();
    assert_eq!(b, SetGenericB::Item(42));
    assert_eq!(b.to_string(), "item: 42");
}

#[test]
fn test_generic_set_diagnostic_helpers() {
    let a = SetGenericA::Item("hello".to_string());
    let report = a.to_report();
    assert_eq!(report.to_string(), "item: hello");

    let a_trans = SetGenericA::Item(100);
    let report_trans: diagweave::report::Report<SetGenericB<i32>> = a_trans.to_report_trans();
    assert_eq!(report_trans.inner(), &SetGenericB::Item(100));
}

#[test]
fn test_lifetime_set() {
    let msg: &'static str = "temporary buffer";
    let s: SetLifetime<'static> = SetLifetime::Borrowed(msg);
    assert_eq!(s.to_string(), "borrowed msg: temporary buffer");

    let sup: SetSuperLifetime<'static> = s.into();
    assert_eq!(sup.to_string(), "borrowed msg: temporary buffer");
}

set! {
    pub SetGenDedupA<T: Display + Debug + std::fmt::Debug + Send + Sync + 'static> = {
        #[display("shared: {0}")]
        Shared(T),
        #[display("non generic shared")]
        NonGenericShared,
        #[display("only A")]
        OnlyA,
    }

    pub SetGenDedupB<T: Display + Debug + std::fmt::Debug + Send + Sync + 'static> = {
        #[display("shared: {0}")]
        Shared(T),
        #[display("non generic shared")]
        NonGenericShared,
        #[display("only B")]
        OnlyB,
    }

    pub SetGenDedupUnion<T: Display + Debug + std::fmt::Debug + Send + Sync + 'static> = SetGenDedupA<T> | SetGenDedupB<T> | {
        #[display("shared: {0}")]
        Shared(T),
        #[display("only Union")]
        OnlyUnion,
    }

    pub SetGenConcreteUnion = SetGenDedupA<u32> | SetGenDedupB<u32> | {
        #[display("only Concrete Union")]
        OnlyConcreteUnion,
    }

    pub SetGenMixA<T: Display + Debug + std::fmt::Debug + Send + Sync + 'static> = {
        #[display("non generic shared")]
        NonGenericShared,
        #[display("only A: {0}")]
        OnlyA(T),
    }

    pub SetGenMixB<U: Display + Debug + std::fmt::Debug + Send + Sync + 'static> = {
        #[display("non generic shared")]
        NonGenericShared,
        #[display("only B: {0}")]
        OnlyB(U),
    }

    pub SetGenMixUnion<T: Display + Debug + std::fmt::Debug + Send + Sync + 'static, U: Display + Debug + std::fmt::Debug + Send + Sync + 'static> = SetGenMixA<T> | SetGenMixB<U>
}

#[test]
fn test_generic_set_deduplication_scenarios() {
    // 1. Same generic type T deduplication
    let a = SetGenDedupA::Shared(100);
    let u: SetGenDedupUnion<i32> = a.into();
    assert_eq!(u.to_string(), "shared: 100");

    let b = SetGenDedupB::Shared(200);
    let u2: SetGenDedupUnion<i32> = b.into();
    assert_eq!(u2.to_string(), "shared: 200");

    let u_only = SetGenDedupUnion::<i32>::OnlyUnion;
    assert_eq!(u_only.to_string(), "only Union");

    // 2. Concrete generic type arguments (u32) deduplication and From conversion
    let a_concrete = SetGenDedupA::Shared(10u32);
    let cu: SetGenConcreteUnion = a_concrete.into();
    assert_eq!(cu.to_string(), "shared: 10");

    let b_concrete = SetGenDedupB::Shared(20u32);
    let cu2: SetGenConcreteUnion = b_concrete.into();
    assert_eq!(cu2.to_string(), "shared: 20");

    let cu3 = SetGenConcreteUnion::OnlyConcreteUnion;
    assert_eq!(cu3.to_string(), "only Concrete Union");

    // 3. Mixed generics: NonGenericShared gets deduplicated across T and U!
    let non_gen_a = SetGenMixA::<i32>::NonGenericShared;
    let mix_u: SetGenMixUnion<i32, String> = non_gen_a.into();
    assert_eq!(mix_u.to_string(), "non generic shared");

    let non_gen_b = SetGenMixB::<String>::NonGenericShared;
    let mix_u2: SetGenMixUnion<i32, String> = non_gen_b.into();
    assert_eq!(mix_u2.to_string(), "non generic shared");
}

// ==========================================
// 1. 复杂泛型与生命周期边界测试
// ==========================================

set! {
    // 多生命周期与生命周期约束 <'a: 'static, 'b: 'static>
    #[derive(Debug, PartialEq)]
    pub MultiLifetimeSet<'a: 'static, 'b: 'static, T: Display + Debug + Send + Sync + 'static> = {
        #[display("ref_a: {0}")]
        RefA(&'a str),
        #[display("ref_b: {0}, t: {1}")]
        RefB(&'b str, T),
    }

    // 默认泛型参数 Set<T = i32>
    #[derive(Debug, PartialEq)]
    pub DefaultGenericSet<T: Display + Debug + Send + Sync + 'static = i32> = {
        #[display("value: {0}")]
        Val(T),
        #[display("default flag")]
        Flag,
    }

    // 关联类型约束 T: Iterator<Item = String>
    #[derive(Debug)]
    pub AssocTypeSet<T> where T: Iterator<Item = String> + Debug + Send + Sync + 'static = {
        #[display("iter item")]
        Iter(T),
    }

    // 常量泛型 Const Generics
    #[derive(Debug, PartialEq)]
    pub ConstGenericSet<T: Display + Debug + Send + Sync + 'static, const N: usize> = {
        #[display("array of len N")]
        Arr([T; N]),
    }
}

#[test]
fn test_multi_lifetime_and_hrtb_and_defaults() {
    let s: &'static str = "hello";
    let set_lt: MultiLifetimeSet<'static, 'static, i32> = MultiLifetimeSet::RefB(s, 100);
    assert_eq!(set_lt.to_string(), "ref_b: hello, t: 100");

    let def_set: DefaultGenericSet<i32> = DefaultGenericSet::Val(42);
    assert_eq!(def_set.to_string(), "value: 42");

    let def_set_custom: DefaultGenericSet<String> = DefaultGenericSet::Val("custom".to_string());
    assert_eq!(def_set_custom.to_string(), "value: custom");

    let const_set: ConstGenericSet<i32, 3> = ConstGenericSet::Arr([1, 2, 3]);
    assert_eq!(const_set.to_string(), "array of len N");
}

// ==========================================
// 2. 字段形态与属性继承测试
// ==========================================

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct CustomField {
    pub value: String,
}

set! {
    // 结构体变体 (Struct Variants) & 多元素元组变体 (Tuple Variants) & 属性穿透 (Doc comments / serde)
    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    pub VariantTypesSet<T: Display + Debug + Send + Sync + 'static> = {
        /// Doc comment for StructVariant
        #[display("named struct x={x}, y={y}")]
        NamedStruct { x: T, y: String },

        /// Doc comment for TupleVariant
        #[display("tuple pair ({0}, {1})")]
        TuplePair(T, String),

        /// Serde passthrough test
        #[serde(rename = "custom_renamed")]
        #[display("renamed variant: {0}")]
        Renamed(String),
    }
}

#[test]
fn test_variant_shapes_and_attribute_passthrough() {
    let v_struct: VariantTypesSet<i32> = VariantTypesSet::NamedStruct {
        x: 10,
        y: "test".to_string(),
    };
    assert_eq!(v_struct.to_string(), "named struct x=10, y=test");

    let v_tuple: VariantTypesSet<i32> = VariantTypesSet::TuplePair(5, "world".to_string());
    assert_eq!(v_tuple.to_string(), "tuple pair (5, world)");

    // Test serde serialization passthrough
    let v_renamed: VariantTypesSet<i32> = VariantTypesSet::Renamed("json".to_string());
    let json_str = serde_json::to_string(&v_renamed).expect("Serialization failed");
    assert!(json_str.contains("custom_renamed"));
}

// ==========================================
// 3. 多层级递归继承 (Multi-level Union)
// ==========================================

set! {
    pub BaseSet = {
        #[display("base item")]
        BaseItem,
    }

    pub MiddleSet = BaseSet | {
        #[display("middle item")]
        MiddleItem,
    }

    pub TopSet = MiddleSet | {
        #[display("top item")]
        TopItem,
    }
}

#[test]
fn test_multi_level_union_implicit_from() {
    let base = BaseSet::BaseItem;
    let middle: MiddleSet = base.into();
    assert_eq!(middle.to_string(), "base item");

    let top: TopSet = middle.into();
    assert_eq!(top.to_string(), "base item");
}

// ==========================================
// 4. 诊断生态与 Thread Safety (Send + Sync) 传播
// ==========================================

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn test_thread_safety_propagation() {
    assert_send_sync::<SetGenericA<i32>>();
    assert_send_sync::<VariantTypesSet<String>>();
    assert_send_sync::<TopSet>();
}

#[derive(Debug, Clone)]
struct RootCause;

impl Display for RootCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "inner root cause")
    }
}

impl std::error::Error for RootCause {}

#[test]
fn test_std_error_and_source_chain() {
    set! {
        #[derive(Clone)]
        pub ErrorChainSet = {
            #[display("error transparent: {0}")]
            Cause(#[from] RootCause),
        }
    }

    let err = ErrorChainSet::Cause(RootCause);
    let report = err.clone().to_report();
    assert_eq!(report.to_string(), "error transparent: inner root cause");

    use std::error::Error;
    let std_err: &dyn Error = &err;
    assert!(std_err.source().is_some());
    assert_eq!(std_err.source().unwrap().to_string(), "inner root cause");
}
