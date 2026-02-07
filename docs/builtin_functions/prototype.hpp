#pragma once
#include <functional>
#include <numeric>
#undef NDEBUG

#include <cassert>
#include <string>
#include <variant>
#include <vector>

struct Any;
template <typename T> using List = std::vector<T>;

struct Any {
  using V = std::variant<bool, int, std::string, List<Any>>;
  V v;

  Any(bool value) : v(value) {}
  Any(int value) : v(value) {}
  Any(std::string value) : v(value) {}
  Any(List<Any> value) : v(value) {}

  Any(const Any &other) : v(other.v) {}
  Any(Any &&other) noexcept : v(std::move(other.v)) {}
};

inline std::string operator"" _ident(const char *s, std::size_t n) {
  return std::string(s, n);
}

/// if<T: Variant>(
///   condition: boolean,
///   then: T,
///   else: T
/// ) -> T
template <typename T0, typename T1>
auto fn_if(bool condition, T0 &&then, T1 &&otherwise)
    -> std::variant<std::decay_t<T0>, std::decay_t<T1>> {
  if (condition) {
    return std::forward<T0>(then);
  } else {
    return std::forward<T1>(otherwise);
  }
}

/// ifs<T: Variant>(
///   condition1: boolean, value1: T,
///   condition2: boolean, value2: T,
///   ...,
///   else: T
/// ) -> T
template <typename Else, typename... Pairs>
auto fn_ifs(Pairs &&...pairs, Else &&else_) {
  using R = std::variant<
      std::decay_t<Else>,
      std::decay_t<std::tuple_element_t<1, std::remove_cvref_t<Pairs>>>...>;

  R out{std::in_place_index<0>, std::forward<Else>(else_)};
  bool matched = false;

  auto tup = std::forward_as_tuple(std::forward<Pairs>(pairs)...);

  auto try_at = [&]<std::size_t I>() {
    auto &[cond, value] = std::get<I>(tup);

    if (!matched && cond) {
      matched = true;
      out = R{std::in_place_index<I + 1>, value};
    }
  };

  // Fold to call try_at for each branch
  [&]<std::size_t... Is>(std::index_sequence<Is...>) {
    (try_at.template operator()<Is>(), ...);
  }(std::make_index_sequence<sizeof...(Pairs)>{});

  return out;
}

/// sum(
///   values: number | number[],
///   ...
/// ) -> number
template <typename... Vs>
  requires(
      std::is_same_v<std::remove_cvref_t<Vs>, std::variant<int, List<int>>> &&
      ...)
auto fn_sum(Vs... values) -> int {
  int sum = 0;

  auto sum_one = [](const auto &v) -> int {
    return std::visit(
        [](const auto &x) -> int {
          using T = std::decay_t<decltype(x)>;

          if constexpr (std::is_same_v<T, int>) {
            return x;
          }
          // List<int>
          else if constexpr (std::is_same_v<T, List<int>>) {
            return std::reduce(x.begin(), x.end(), 0, std::plus<int>());
          } else {
            static_assert(!sizeof(T), "unhandled variant alternative");
          }
        },
        v);
  };

  ((sum += sum_one(values)), ...);
  return sum;
}

/// length(
///   value: string | any[]
/// ) -> number
auto fn_length(std::variant<std::string, List<Any>> &&value) -> size_t {
  return std::visit(
      [](const auto &x) -> size_t {
        using T = std::decay_t<decltype(x)>;
        if constexpr (std::is_same_v<T, std::string>) {
          return x.size();
        } else if constexpr (std::is_same_v<T, List<Any>>) {
          return x.size();
        } else {
          static_assert(!sizeof(T), "unhandled variant alternative");
        }
      },
      std::forward<std::variant<std::string, List<Any>>>(value));
}

/// let<T: Plain, R: Plain>(
///   var: `Ident`,
///   value: T,
//    expr: (var: T) -> R,
/// ) -> R
template <typename T, typename R>
auto fn_let(std::string_view var, T &&value, std::function<R(T)> &&expr) -> R {
  return expr(std::forward<T>(value));
}

/// lets<T: Plain, R: Plain>(
///   var1: `Ident`, value1: T,
///   var2: `Ident`, value2: T,
///   ...,
///   expr: (var1: T, var2: T, ..., varN: T) -> R,
/// ) -> R
template <typename F, typename... Pairs> auto fn_lets(Pairs &&...pairs, F expr) {
  auto values =
      std::forward_as_tuple(std::get<1>(std::forward<Pairs>(pairs))...);
  return std::apply(std::forward<F>(expr), values);
}
