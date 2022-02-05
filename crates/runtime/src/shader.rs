use naga::back::wgsl::WriterFlags;
use naga::{valid::ModuleInfo, Module};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::{
    borrow::Cow, collections::HashSet, marker::Copy, ops::Deref, path::PathBuf, str::FromStr,
};
use wgpu::util::make_spirv;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShaderReflectError {
    #[error("Wgsl ParseError: {0:?}")]
    WgslParse(#[from] naga::front::wgsl::ParseError),
    #[error("GLSL Parse Error: {0:?}")]
    GlslParse(Vec<naga::front::glsl::Error>),
    #[error(transparent)]
    SpirVParse(#[from] naga::front::spv::Error),
    #[error(transparent)]
    Validation(#[from] naga::WithSpan<naga::valid::ValidationError>),
}

/// A shader, as defined by its [`ShaderSource`] and [`ShaderStage`](naga::ShaderStage)
/// This is an "unprocessed" shader. It can contain preprocessor directives.
#[derive(Debug, Clone)]
pub struct Shader {
    source: Source,
    import_path: Option<ShaderImport>,
    imports: Vec<ShaderImport>,
}

impl Shader {
    pub fn from_wgsl(source: impl Into<Cow<'static, str>>) -> Shader {
        let source = source.into();
        Shader {
            imports: SHADER_IMPORT_PROCESSOR.get_imports_from_str(&source),
            source: Source::Wgsl(source),
            import_path: None,
        }
    }

    pub fn from_glsl(source: impl Into<Cow<'static, str>>, stage: naga::ShaderStage) -> Shader {
        let source = source.into();
        Shader {
            imports: SHADER_IMPORT_PROCESSOR.get_imports_from_str(&source),
            source: Source::Glsl(source, stage),
            import_path: None,
        }
    }

    pub fn from_spirv(source: impl Into<Cow<'static, [u8]>>) -> Shader {
        Shader {
            imports: Vec::new(),
            source: Source::SpirV(source.into()),
            import_path: None,
        }
    }

    pub fn set_import_path<P: Into<String>>(&mut self, import_path: P) {
        self.import_path = Some(ShaderImport::Custom(import_path.into()));
    }

    pub fn with_import_path<P: Into<String>>(mut self, import_path: P) -> Self {
        self.set_import_path(import_path);
        self
    }

    #[inline]
    pub fn import_path(&self) -> Option<&ShaderImport> {
        self.import_path.as_ref()
    }

    pub fn imports(&self) -> impl ExactSizeIterator<Item = &ShaderImport> {
        self.imports.iter()
    }
}

#[derive(Debug, Clone)]
pub enum Source {
    Wgsl(Cow<'static, str>),
    Glsl(Cow<'static, str>, naga::ShaderStage),
    SpirV(Cow<'static, [u8]>),
    // TODO: consider the following
    // PrecompiledSpirVMacros(HashMap<HashSet<String>, Vec<u32>>)
    // NagaModule(Module) ... Module impls Serialize/Deserialize
}

/// A processed [Shader]. This cannot contain preprocessor directions. It must be "ready to compile"
#[derive(PartialEq, Eq, Debug)]
pub enum ProcessedShader {
    Wgsl(Cow<'static, str>),
    Glsl(Cow<'static, str>, naga::ShaderStage),
    SpirV(Cow<'static, [u8]>),
}

impl ProcessedShader {
    pub fn get_wgsl_source(&self) -> Option<&str> {
        if let ProcessedShader::Wgsl(source) = self {
            Some(source)
        } else {
            None
        }
    }
    pub fn get_glsl_source(&self) -> Option<&str> {
        if let ProcessedShader::Glsl(source, _stage) = self {
            Some(source)
        } else {
            None
        }
    }

    pub fn reflect(&self) -> Result<ShaderReflection, ShaderReflectError> {
        let module = match &self {
            // TODO: process macros here
            ProcessedShader::Wgsl(source) => naga::front::wgsl::parse_str(source)?,
            ProcessedShader::Glsl(source, shader_stage) => {
                let mut parser = naga::front::glsl::Parser::default();
                parser
                    .parse(&naga::front::glsl::Options::from(*shader_stage), source)
                    .map_err(ShaderReflectError::GlslParse)?
            }
            ProcessedShader::SpirV(source) => naga::front::spv::parse_u8_slice(
                source,
                &naga::front::spv::Options {
                    adjust_coordinate_space: false,
                    ..naga::front::spv::Options::default()
                },
            )?,
        };
        let module_info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::default(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)?;

        Ok(ShaderReflection {
            module,
            module_info,
        })
    }

    pub fn get_module_descriptor(
        &self,
    ) -> Result<wgpu::ShaderModuleDescriptor, AsModuleDescriptorError> {
        Ok(wgpu::ShaderModuleDescriptor {
            label: None,
            source: match self {
                ProcessedShader::Wgsl(source) => {
                    #[cfg(debug_assertions)]
                    // This isn't neccessary, but catches errors early during hot reloading of invalid wgsl shaders.
                    // Eventually, wgpu will have features that will make this unneccessary like compilation info
                    // or error scopes, but until then parsing the shader twice during development the easiest solution.
                    let _ = self.reflect()?;

                    wgpu::ShaderSource::Wgsl(source.clone())
                }
                ProcessedShader::Glsl(_source, _stage) => {
                    let reflection = self.reflect()?;
                    // TODO: it probably makes more sense to convert this to spirv, but as of writing
                    // this comment, naga's spirv conversion is broken
                    let wgsl = reflection.get_wgsl()?;
                    wgpu::ShaderSource::Wgsl(wgsl.into())
                }
                ProcessedShader::SpirV(source) => make_spirv(source),
            },
        })
    }
}

#[derive(Error, Debug)]
pub enum AsModuleDescriptorError {
    #[error(transparent)]
    ShaderReflectError(#[from] ShaderReflectError),
    #[error(transparent)]
    WgslConversion(#[from] naga::back::wgsl::Error),
    #[error(transparent)]
    SpirVConversion(#[from] naga::back::spv::Error),
}

pub struct ShaderReflection {
    pub module: Module,
    pub module_info: ModuleInfo,
}

impl ShaderReflection {
    pub fn get_spirv(&self) -> Result<Vec<u32>, naga::back::spv::Error> {
        naga::back::spv::write_vec(
            &self.module,
            &self.module_info,
            &naga::back::spv::Options {
                flags: naga::back::spv::WriterFlags::empty(),
                ..naga::back::spv::Options::default()
            },
            None,
        )
    }

    pub fn get_wgsl(&self) -> Result<String, naga::back::wgsl::Error> {
        naga::back::wgsl::write_string(&self.module, &self.module_info, WriterFlags::EXPLICIT_TYPES)
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ProcessShaderError {
    #[error("Too many '# endif' lines. Each endif should be preceded by an if statement.")]
    TooManyEndIfs,
    #[error(
        "Not enough '# endif' lines. Each if statement should be followed by an endif statement."
    )]
    NotEnoughEndIfs,
    #[error("This Shader's format does not support processing shader defs.")]
    ShaderFormatDoesNotSupportShaderDefs,
    #[error("This Shader's formatdoes not support imports.")]
    ShaderFormatDoesNotSupportImports,
    #[error("Unresolved import: {0:?}.")]
    UnresolvedImport(ShaderImport),
    #[error("The shader import {0:?} does not match the source file type. Support for this might be added in the future.")]
    MismatchedImportFormat(ShaderImport),
}

pub struct ShaderImportProcessor {
    import_asset_path_regex: Regex,
    import_custom_path_regex: Regex,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum ShaderImport {
    AssetPath(String),
    Custom(String),
}

impl Default for ShaderImportProcessor {
    fn default() -> Self {
        Self {
            import_asset_path_regex: Regex::new(r#"^\s*#\s*import\s*"(.+)""#).unwrap(),
            import_custom_path_regex: Regex::new(r"^\s*#\s*import\s*(.+)").unwrap(),
        }
    }
}

impl ShaderImportProcessor {
    pub fn get_imports(&self, shader: &Shader) -> Vec<ShaderImport> {
        match &shader.source {
            Source::Wgsl(source) => self.get_imports_from_str(source),
            Source::Glsl(source, _stage) => self.get_imports_from_str(source),
            Source::SpirV(_source) => Vec::new(),
        }
    }

    pub fn get_imports_from_str(&self, shader: &str) -> Vec<ShaderImport> {
        let mut imports = Vec::new();
        for line in shader.lines() {
            if let Some(cap) = self.import_asset_path_regex.captures(line) {
                let import = cap.get(1).unwrap();
                imports.push(ShaderImport::AssetPath(import.as_str().to_string()));
            } else if let Some(cap) = self.import_custom_path_regex.captures(line) {
                let import = cap.get(1).unwrap();
                imports.push(ShaderImport::Custom(import.as_str().to_string()));
            }
        }

        imports
    }
}

pub static SHADER_IMPORT_PROCESSOR: Lazy<ShaderImportProcessor> =
    Lazy::new(ShaderImportProcessor::default);

static SHADER_CURRENT_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Hash, Debug, Copy, Clone, PartialEq, PartialOrd, Eq)]
pub struct ShaderHandle(pub u32);

impl ShaderHandle {
    pub fn new() -> Self {
        let current = SHADER_CURRENT_ID.fetch_add(1, Ordering::SeqCst);
        Self(current)
    }
}

pub struct ShaderProcessor {
    ifdef_regex: Regex,
    ifndef_regex: Regex,
    else_regex: Regex,
    endif_regex: Regex,
}

impl Default for ShaderProcessor {
    fn default() -> Self {
        Self {
            ifdef_regex: Regex::new(r"^\s*#\s*ifdef\s*([\w|\d|_]+)").unwrap(),
            ifndef_regex: Regex::new(r"^\s*#\s*ifndef\s*([\w|\d|_]+)").unwrap(),
            else_regex: Regex::new(r"^\s*#\s*else").unwrap(),
            endif_regex: Regex::new(r"^\s*#\s*endif").unwrap(),
        }
    }
}

impl ShaderProcessor {
    pub fn process(
        &self,
        shader: &Shader,
        shader_defs: &[String],
        shaders: &HashMap<ShaderHandle, Shader>,
        import_handles: &HashMap<ShaderImport, ShaderHandle>,
    ) -> Result<ProcessedShader, ProcessShaderError> {
        let shader_str = match &shader.source {
            Source::Wgsl(source) => source.deref(),
            Source::Glsl(source, _stage) => source.deref(),
            Source::SpirV(source) => {
                if shader_defs.is_empty() {
                    return Ok(ProcessedShader::SpirV(source.clone()));
                } else {
                    return Err(ProcessShaderError::ShaderFormatDoesNotSupportShaderDefs);
                }
            }
        };

        let shader_defs_unique = HashSet::<String>::from_iter(shader_defs.iter().cloned());
        let mut scopes = vec![true];
        let mut final_string = String::new();
        for line in shader_str.lines() {
            if let Some(cap) = self.ifdef_regex.captures(line) {
                let def = cap.get(1).unwrap();
                scopes.push(*scopes.last().unwrap() && shader_defs_unique.contains(def.as_str()));
            } else if let Some(cap) = self.ifndef_regex.captures(line) {
                let def = cap.get(1).unwrap();
                scopes.push(*scopes.last().unwrap() && !shader_defs_unique.contains(def.as_str()));
            } else if self.else_regex.is_match(line) {
                let mut is_parent_scope_truthy = true;
                if scopes.len() > 1 {
                    is_parent_scope_truthy = scopes[scopes.len() - 2];
                }
                if let Some(last) = scopes.last_mut() {
                    *last = is_parent_scope_truthy && !*last;
                }
            } else if self.endif_regex.is_match(line) {
                scopes.pop();
                if scopes.is_empty() {
                    return Err(ProcessShaderError::TooManyEndIfs);
                }
            } else if let Some(cap) = SHADER_IMPORT_PROCESSOR
                .import_asset_path_regex
                .captures(line)
            {
                let import = ShaderImport::AssetPath(cap.get(1).unwrap().as_str().to_string());
                self.apply_import(
                    import_handles,
                    shaders,
                    &import,
                    shader,
                    shader_defs,
                    &mut final_string,
                )?;
            } else if let Some(cap) = SHADER_IMPORT_PROCESSOR
                .import_custom_path_regex
                .captures(line)
            {
                let import = ShaderImport::Custom(cap.get(1).unwrap().as_str().to_string());
                self.apply_import(
                    import_handles,
                    shaders,
                    &import,
                    shader,
                    shader_defs,
                    &mut final_string,
                )?;
            } else if *scopes.last().unwrap() {
                final_string.push_str(line);
                final_string.push('\n');
            }
        }

        if scopes.len() != 1 {
            return Err(ProcessShaderError::NotEnoughEndIfs);
        }

        let processed_source = Cow::from(final_string);

        match &shader.source {
            Source::Wgsl(_source) => Ok(ProcessedShader::Wgsl(processed_source)),
            Source::Glsl(_source, stage) => Ok(ProcessedShader::Glsl(processed_source, *stage)),
            Source::SpirV(_source) => {
                unreachable!("SpirV has early return");
            }
        }
    }

    fn apply_import(
        &self,
        import_handles: &HashMap<ShaderImport, ShaderHandle>,
        shaders: &HashMap<ShaderHandle, Shader>,
        import: &ShaderImport,
        shader: &Shader,
        shader_defs: &[String],
        final_string: &mut String,
    ) -> Result<(), ProcessShaderError> {
        let imported_shader = import_handles
            .get(import)
            .and_then(|handle| shaders.get(handle))
            .ok_or_else(|| ProcessShaderError::UnresolvedImport(import.clone()))?;
        let imported_processed =
            self.process(imported_shader, shader_defs, shaders, import_handles)?;

        match &shader.source {
            Source::Wgsl(_) => {
                if let ProcessedShader::Wgsl(import_source) = &imported_processed {
                    final_string.push_str(import_source);
                } else {
                    return Err(ProcessShaderError::MismatchedImportFormat(import.clone()));
                }
            }
            Source::Glsl(_, _) => {
                if let ProcessedShader::Glsl(import_source, _) = &imported_processed {
                    final_string.push_str(import_source);
                } else {
                    return Err(ProcessShaderError::MismatchedImportFormat(import.clone()));
                }
            }
            Source::SpirV(_) => {
                return Err(ProcessShaderError::ShaderFormatDoesNotSupportImports);
            }
        }

        Ok(())
    }
}
